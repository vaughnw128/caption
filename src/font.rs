use cosmic_text::{FontSystem, fontdb};

/// Embedded font bytes bundled at compile time.
const IFUNNY_TTF: &[u8] = include_bytes!("../fonts/ifunny.ttf");
const EMOJI_TTF: &[u8] = include_bytes!("../fonts/emoji.ttf");
const JP_TTF: &[u8] = include_bytes!("../fonts/jp.ttf");

pub fn build_font_system(extra_fonts: &[Vec<u8>]) -> FontSystem {
    let mut db = fontdb::Database::new();

    // order priority
    db.load_font_data(IFUNNY_TTF.to_vec());
    db.load_font_data(EMOJI_TTF.to_vec());
    db.load_font_data(JP_TTF.to_vec());

    for data in extra_fonts {
        db.load_font_data(data.clone());
    }

    FontSystem::new_with_locale_and_db("en-US".into(), db)
}
