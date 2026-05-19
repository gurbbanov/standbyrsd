use fluent::{FluentArgs, FluentBundle, FluentResource};
use serde::{Deserialize, Serialize};
use unic_langid::langid;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
pub enum Locale {
    #[default]
    En,
    Ru,
    Az,
    Tr,
    Ka,
    De,
    Es,
    Fr,
    Ja,
    Kk,
    Zh,
}

impl Locale {
    pub fn as_str(&self) -> &str {
        match self {
            Locale::En => "en",
            Locale::Ru => "ru",
            Locale::Az => "az",
            Locale::Tr => "tr",
            Locale::Ka => "ka",
            Locale::De => "de",
            Locale::Es => "es",
            Locale::Fr => "fr",
            Locale::Ja => "ja",
            Locale::Kk => "kk",
            Locale::Zh => "zh",
        }
    }

    pub fn all() -> Vec<Locale> {
        vec![
            Locale::En,
            Locale::Ru,
            Locale::Az,
            Locale::Tr,
            Locale::Ka,
            Locale::De,
            Locale::Es,
            Locale::Fr,
            Locale::Ja,
            Locale::Kk,
            Locale::Zh,
        ]
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Locale::En => write!(f, "english"),
            Locale::Ru => write!(f, "русский"),
            Locale::Az => write!(f, "azərbaycanca"),
            Locale::Tr => write!(f, "türkçe"),
            Locale::Ka => write!(f, "ქართული"),
            Locale::De => write!(f, "deutsch"),
            Locale::Es => write!(f, "español"),
            Locale::Fr => write!(f, "français"),
            Locale::Ja => write!(f, "日本語"),
            Locale::Kk => write!(f, "қазақша"),
            Locale::Zh => write!(f, "中文"),
        }
    }
}

pub struct L10n {
    pub bundle: FluentBundle<FluentResource>,
}

impl L10n {
    pub fn new(locale: &str) -> Self {
        let ftl = match locale {
            "ru" => include_str!("../locales/ru/main.ftl"),
            "az" => include_str!("../locales/az/main.ftl"),
            "tr" => include_str!("../locales/tr/main.ftl"),
            "ka" => include_str!("../locales/ka/main.ftl"),
            "de" => include_str!("../locales/de/main.ftl"),
            "es" => include_str!("../locales/es/main.ftl"),
            "fr" => include_str!("../locales/fr/main.ftl"),
            "ja" => include_str!("../locales/ja/main.ftl"),
            "kk" => include_str!("../locales/kk/main.ftl"),
            "zh" => include_str!("../locales/zh/main.ftl"),
            _ => include_str!("../locales/en/main.ftl"),
        };

        let res = FluentResource::try_new(ftl.to_string()).unwrap();
        let langid = locale.parse().unwrap_or(langid!("en"));
        let mut bundle = FluentBundle::new(vec![langid]);
        bundle.add_resource(res).unwrap();

        Self { bundle }
    }

    pub fn get(&self, key: &str) -> String {
        let msg = self.bundle.get_message(key).unwrap();
        let pattern = msg.value().unwrap();
        let mut errors = vec![];
        self.bundle
            .format_pattern(pattern, None, &mut errors)
            .to_string()
    }

    pub fn get_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        let msg = self.bundle.get_message(key).unwrap();
        let pattern = msg.value().unwrap();
        let mut fluent_args = FluentArgs::new();
        for (k, v) in args {
            fluent_args.set(*k, *v);
        }
        let mut errors = vec![];
        self.bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors)
            .to_string()
    }
}
