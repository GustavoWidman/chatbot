macro_rules! config {
    ($data:expr) => {{
        let config = $data.config.write().await;
        config.downgrade().clone()
    }};
}

pub(crate) use config;
