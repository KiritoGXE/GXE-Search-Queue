use gxe_search_queue::core::{Queue, Task};

#[test]
fn test_queue_push_and_pop() {
    let mut q = Queue::new();
    assert!(q.is_empty());

    q.push(Task::new("1", "search query A"));
    assert_eq!(q.len(), 1);

    let t = q.pop().unwrap();
    assert_eq!(t.id, "1");
    assert!(q.is_empty());
}
