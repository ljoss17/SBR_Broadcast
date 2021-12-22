#[macro_use]
pub mod log {
    #[macro_export]
    macro_rules! my_print {
        ($e:expr) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            println!("{:?} - {}", now, $e);
        };
    }
}
