use minimal_async_runtime::{mini_rt, sleep};
use std::time::Duration;

async fn task_one() {
    println!("task one: start");
    sleep(Duration::from_secs(1)).await;
    println!("task one: done");
}

async fn task_two() {
    println!("task two: start");
    sleep(Duration::from_secs(2)).await;
    println!("task two: done");
}

mini_rt! {
    println!("Runtime started...");
    task_one().await;
    task_two().await;
}
