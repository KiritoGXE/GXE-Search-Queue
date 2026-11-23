use crate::core::{MediaType, QueueJob};

#[derive(Debug, Clone)]
pub struct SchedulerPolicy {
    pub minimum_interval_seconds: i64,
    pub max_consecutive_category_searches: u32,
}

impl Default for SchedulerPolicy {
    fn default() -> Self {
        Self {
            minimum_interval_seconds: 10 * 60,
            max_consecutive_category_searches: 2,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SchedulerHistory {
    pub previous_start_at: Option<i64>,
    pub previous_media_type: Option<MediaType>,
    pub consecutive_same_type: u32,
    pub previous_series_key: Option<String>,
    pub active_search: bool,
}

impl SchedulerPolicy {
    /// Choose from jobs already known to be eligible and not in flight.
    /// Request priority wins within the chosen class; episode chronology and
    /// series rotation are enforced before selecting an episode.
    pub fn choose<'a>(
        &self,
        jobs: &'a [QueueJob],
        history: &SchedulerHistory,
    ) -> Option<&'a QueueJob> {
        let movies_exist = jobs.iter().any(|job| job.media_type == MediaType::Movie);
        let episodes_exist = jobs.iter().any(|job| job.media_type == MediaType::Episode);

        let forced_type = if movies_exist
            && episodes_exist
            && history.consecutive_same_type >= self.max_consecutive_category_searches
        {
            match history.previous_media_type? {
                MediaType::Movie => Some(MediaType::Episode),
                MediaType::Episode => Some(MediaType::Movie),
            }
        } else {
            None
        };

        let candidates: Vec<&QueueJob> = jobs
            .iter()
            .filter(|job| forced_type.is_none_or(|kind| job.media_type == kind))
            .filter(|job| {
                if job.media_type != MediaType::Episode {
                    return true;
                }
                !jobs.iter().any(|other| {
                    other.media_type == MediaType::Episode
                        && other.series_key == job.series_key
                        && match (&other.episode_sort_key, &job.episode_sort_key) {
                            (Some(earlier), Some(current)) => earlier < current,
                            (None, Some(_)) => true,
                            _ => false,
                        }
                })
            })
            .collect();

        let has_other_series = history
            .previous_series_key
            .as_ref()
            .is_some_and(|last_series| {
                candidates.iter().any(|job| {
                    job.media_type == MediaType::Episode
                        && job
                            .series_key
                            .as_ref()
                            .is_some_and(|series| series != last_series)
                })
            });

        candidates
            .into_iter()
            .filter(|job| {
                !has_other_series
                    || job.media_type != MediaType::Episode
                    || job.series_key != history.previous_series_key
            })
            .min_by(|a, b| {
                b.priority
                    .cmp(&a.priority)
                    .then_with(|| match (a.media_type, b.media_type) {
                        (MediaType::Episode, MediaType::Episode) => a
                            .episode_sort_key
                            .cmp(&b.episode_sort_key)
                            .then_with(|| a.first_requested_at.cmp(&b.first_requested_at)),
                        _ => a.first_requested_at.cmp(&b.first_requested_at),
                    })
            })
    }
}

pub fn no_result_delay_seconds(attempt: u32) -> i64 {
    const DELAYS: [i64; 5] = [3600, 6 * 3600, 24 * 3600, 3 * 24 * 3600, 7 * 24 * 3600];
    if attempt == 0 {
        return DELAYS[0];
    }
    if attempt <= DELAYS.len() as u32 {
        return DELAYS[attempt as usize - 1];
    }
    // Attempt six is a seven-day suspension. Later attempts repeat the
    // approved cycle from its one-hour delay.
    if attempt == 6 {
        DELAYS[4]
    } else {
        DELAYS[(attempt as usize - 7) % DELAYS.len()]
    }
}
