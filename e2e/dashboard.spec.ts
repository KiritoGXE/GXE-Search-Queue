import { expect, test } from '@playwright/test';

const now = Math.floor(Date.now() / 1000);
const jobs = [
  { id: 'movie-1', media_type: 'movie', media_key: 'tmdb:101', title: 'Dune: Part Two', series_key: null, episode_sort_key: null, external_manager_id: 11, state: 'queued', priority: 0, eligible_at: null, next_attempt_at: null, no_result_attempts: 0, first_requested_at: now - 7200 },
  { id: 'episode-1', media_type: 'episode', media_key: 'tvdb:201', title: 'Arcane — Welcome to the Playground S01E01', series_key: 'tvdb:99', episode_sort_key: '1-1', external_manager_id: 21, state: 'queued', priority: 0, eligible_at: null, next_attempt_at: null, no_result_attempts: 0, first_requested_at: now - 6800 },
  { id: 'episode-2', media_type: 'episode', media_key: 'tvdb:202', title: 'Arcane — Some Mysteries Are Better Left Unsolved S01E02', series_key: 'tvdb:99', episode_sort_key: '1-2', external_manager_id: 22, state: 'retry_wait', priority: 0, eligible_at: null, next_attempt_at: now + 1800, no_result_attempts: 1, first_requested_at: now - 6500 },
];

const status = {
  paused: false,
  counts: { movies: 1, episodes: 2, suspended: 0 },
  outcomes_90d: { completed: 14, no_results: 3, technical_errors: 1 },
  active_search: null,
  services: ['seerr', 'radarr', 'sonarr', 'prowlarr'].map(service => ({ service, available: true, checked_at: now, detail: null })),
  active_cooldowns: 0,
};

async function mockApi(page: import('@playwright/test').Page) {
  await page.route('**/api/v1/**', async route => {
    const url = new URL(route.request().url());
    const path = url.pathname.replace('/api/v1/', '');
    let body: unknown = {};
    if (path === 'status') body = status;
    else if (path === 'queue') body = { items: jobs };
    else if (path === 'history') body = { items: [{ id: 'run-1', job_id: 'job-older', state: 'completed', started_at: now - 3600, completed_at: now - 3500, detail: null, title: 'Past feature film', media_type: 'movie' }] };
    else if (path === 'events') body = { items: [] };
    else if (path === 'cooldowns') body = { items: [] };
    else if (path === 'settings') body = { minimum_interval_seconds: 600, queue_capacity: 0, search_timeout_seconds: 900, movie_theatrical_delay_days: 0 };
    else if (path === 'indexers') body = { checked_at: now, period_started_at: now - 86400, indexers: [{ id: 7, name: 'Example Indexer', enabled: true, protocol: 'torrent', privacy: 'public', disabled_until: null, most_recent_failure: null, queries_24h: 18, grabs_24h: 2, failed_queries_24h: 1, average_response_time_ms_24h: 430 }] };
    else if (path.endsWith('/details')) body = { job: jobs[0], requests: [{ external_id: 'request-1', requester_name: 'Test User', media_type: 'movie', created_at: now - 7200 }], runs: [] };
    return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(body) });
  });
}

async function signIn(page: import('@playwright/test').Page) {
  await mockApi(page);
  await page.goto('/');
  await page.getByLabel('Token amministratore').fill('test-token');
  await page.getByRole('button', { name: /Apri la console/ }).click();
  await expect(page.getByRole('heading', { name: 'Overview.' })).toBeVisible();
  await expect(page.locator('dialog[open]')).toHaveCount(0);
}

test('naviga le quattro sezioni e mostra stati reali senza simulare funzioni indexer', async ({ page }) => {
  await signIn(page);
  await expect(page.getByText('Servizi richiesti raggiungibili')).toBeVisible();
  await page.screenshot({ path: '/tmp/gxe-overview.png', fullPage: true });
  await page.getByRole('link', { name: 'Queue', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Queue.' })).toBeVisible();
  await expect(page.getByRole('button', { name: /Arcane/ })).toBeVisible();
  await page.getByRole('button', { name: /Arcane/ }).click();
  await page.getByRole('button', { name: /Stagione 01/ }).click();
  await expect(page.getByText('Welcome to the Playground')).toBeVisible();
  await page.getByRole('link', { name: 'Indexers', exact: true }).click();
  await expect(page.getByText(/non dimostra una distanza minima per singolo indexer/)).toBeVisible();
  await expect(page.getByText('Example Indexer')).toBeVisible();
  await expect(page.getByText('430 ms')).toBeVisible();
  await page.getByRole('link', { name: 'Settings', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Settings.' })).toBeVisible();
  await expect(page.getByLabel('Intervallo minimo')).toHaveValue('600');
  await page.getByRole('link', { name: 'Queue', exact: true }).click();
  await page.getByRole('button', { name: /Search history/ }).click();
  await expect(page.getByText('Past feature film')).toBeVisible();
  await page.getByRole('button', { name: /Manifest/ }).click();
  await page.getByRole('button', { name: 'Rimuovi Dune: Part Two' }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Rimuovere questo elemento?' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
});

test('responsive mobile mantiene leggibili navigazione e righe coda', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await signIn(page);
  await page.getByRole('link', { name: 'Queue', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'Queue.' })).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
  expect(overflow).toBe(false);
  await expect(page.getByRole('button', { name: 'Dune: Part Two', exact: true })).toBeVisible();
  await page.screenshot({ path: '/tmp/gxe-queue-mobile.png', fullPage: true });
});
