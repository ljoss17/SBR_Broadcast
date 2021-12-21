#[macro_use]
pub mod log {
    #[macro_export]
    macro_rules! my_print {
        ($e:expr) => {
            use std::time::{SystemTime, UNIX_EPOCH};

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            println!("{:?} - {}", now, $e);
        };
    }
}
