use std::sync::LazyLock;

#[derive(Debug, Clone)]

pub struct EnvConfigs {
    pub alchemy_api_url: String,
    pub mnemonic: String,
}

pub static ENV_CONFIGS: LazyLock<EnvConfigs> = LazyLock::new(|| {
    dotenvy::dotenv().ok();

    EnvConfigs {
        alchemy_api_url: std::env::var("ALCHEMY_API_URL").expect("ALCHEMY_API_URL must be set"),
        mnemonic: std::env::var("MNEMONIC").expect("MNEMONIC must be set"),
    }
});
