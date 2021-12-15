use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

#[macro_export]
macro_rules! my_print {
    ($e:expr) => {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        println!("{:?} - {}", now, $e);
    };
}

#[tokio::main]
async fn main() {
    my_print!("First task");
    test_cloned_mutex_values().await;
    my_print!("Second task");
    test_mutex_values().await;
}

async fn test_cloned_mutex_values() {
    let m: Arc<Mutex<u32>> = Arc::new(Mutex::new(1));

    let mut m1 = m.lock().await.clone();

    let mut v = vec![];
    v.push(tokio::spawn(async move {
        my_print!("Inside first task");
        m1 += 2;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        my_print!(format!("m1: {}", m1));
    }));
    let mut m2 = m.lock().await.clone();
    v.push(tokio::spawn(async move {
        my_print!("Inside second task");
        m2 *= 2;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        my_print!(format!("m2: {}", m2));
    }));
    my_print!(format!("m : {}", m.lock().await));

    futures::future::join_all(v).await;
}

async fn test_mutex_values() {
    let m: Arc<Mutex<u32>> = Arc::new(Mutex::new(1));

    let m1 = m.clone();

    let mut v = vec![];
    v.push(tokio::spawn(async move {
        my_print!("Inside first task");
        let mut t = m1.lock().await;
        my_print!("Got lock m1");
        *t += 2;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        drop(t);
        my_print!(format!("m1: {}", m1.lock().await));
    }));
    let m2 = m.clone();
    v.push(tokio::spawn(async move {
        my_print!("Inside second task");
        let mut t = m2.lock().await;
        my_print!("Got lock m2");
        *t *= 2;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        drop(t);
        my_print!(format!("m2: {}", m2.lock().await));
    }));
    my_print!(format!("m : {}", m.lock().await));

    futures::future::join_all(v).await;
}
