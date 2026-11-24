//! The settings window's logic that isn't drawing: the search index and its Turkish folding,
//! fullscreen exception names, the timer presets, team crest stand-ins (design:
//! docs/superpowers/designs/2026-10-20-settings-v2/notes.md §5.12, §5.16, §9).

use crate::activities::buds::Paired;
use crate::i18n::{n_, tr, trf};
use crate::settings::BudsDevice;

/// Sidebar order; `SearchHit::page` and the window's `Store.page` index this.
pub const PAGES: [&str; 15] = [n_("Görünüm"), n_("Genel"), n_("Medya"), n_("Zamanlayıcı"), n_("Wireless Buds"), n_("Canlı maç"), n_("Tam ekran"), n_("Hakkında"), n_("Weather"), n_("Oyunlar"), n_("Dijital Denge"), n_("Pano"), n_("Raf"), n_("Ana sayfa"), n_("System Events")];

/// One searchable row: page, row key (the window scrolls to it), section, title, description,
/// extra words people might type.
pub struct Entry {
    pub page: u8,
    pub row: &'static str,
    pub section: &'static str,
    pub title: &'static str,
    pub desc: &'static str,
    pub keywords: &'static str,
}

const fn e(page: u8, row: &'static str, section: &'static str, title: &'static str, desc: &'static str, keywords: &'static str) -> Entry {
    Entry { page, row, section, title, desc, keywords }
}

/// Every setting row, in page order.
pub const INDEX: &[Entry] = &[
    e(0, "pill-size", n_("Pil"), n_("Pil boyutu"), n_("Küçük (%78), Normal (%88)"), "küçük normal boyut"),
    e(0, "pill-offset", n_("Pil"), n_("Konum"), n_("Görev çubuğunun ortasından sola ya da sağa"), "ortala kaydır sol sağ yer"),
    e(0, "pill-opacity", n_("Pil"), n_("Zemin opaklığı"), n_("Pilin saydamlığı"), "saydamlık şeffaflık opacity zemin"),
    e(0, "card-size", n_("Kart"), n_("Kart boyutu"), n_("Kompakt ya da büyük"), "kompakt büyük boyut"),
    e(0, "card-blur", n_("Kart"), n_("Bulanıklık"), n_("Kartın arkasındaki masaüstü bulanıklaşır"), "blur saydam akrilik malzeme"),
    e(0, "speed", n_("Hareket"), n_("Animasyon hızı"), n_("Pilin büyüyüp küçülmesi, kartın açılması, içerik geçişleri"), "animasyon hız yavaş hızlı kapalı"),
    e(0, "marquee", n_("Hareket"), n_("Kayan başlık"), n_("Pile sığmayan şarkı adı"), "kayma marquee başlık"),
    e(0, "marquee-speed", n_("Hareket"), n_("Kayma hızı"), n_("Kayan başlığın hızı"), "kayma hız marquee"),
    e(1, "autostart", n_("Başlangıç"), n_("Windows ile başlat"), n_("Oturum açınca Swift Tab kendiliğinden çalışır"), "başlangıç açılış otomatik"),
    e(11, "clipboard-text", n_("Neleri tutsun"), n_("Metin"), n_("Kopyaladığın yazılar"), "pano metin yazı text"),
    e(11, "clipboard-links", n_("Neleri tutsun"), n_("Bağlantılar"), n_("Web adresleri"), "pano bağlantı link url"),
    e(11, "clipboard-images", n_("Neleri tutsun"), n_("Görseller"), n_("Resimler ve ekran görüntüleri"), "pano görsel resim image"),
    e(11, "clipboard-rich", n_("Neleri tutsun"), n_("Biçimlendirme"), n_("Kapalıyken yalnızca yazı kalır"), "pano biçim kalın rich html"),
    e(11, "clipboard-stash", n_("Pano"), n_("Pano geçmişi"), n_("Kopyaladığın son 20 şeyi tutar; kapatınca kaydetmeyi bırakır"), "pano clipboard kopyala geçmiş"),
    e(11, "clipboard-popup", n_("Pano"), n_("Kopyalayınca pilde göster"), n_("Kopyaladığında pil kısa süre öne çıkar"), "pano clipboard pop açıl göster"),
    e(11, "clipboard-pinned", n_("Behavior"), n_("Always show"), n_("Even when nothing is copied"), "clipboard always show pinned empty hep görünür sabit boş pano"),
    e(12, "shelf-on", n_("Raf"), n_("Rafı göster"), n_("Dosyaları pile bırakıp tutar; kapatınca raf gizlenir"), "raf shelf dosya bırak tut"),
    e(12, "shelf-pinned", n_("Pill"), n_("Always show"), n_("Even when the shelf is empty"), "shelf always show pinned empty hep görünür sabit boş raf"),
    e(1, "close-on-leave", n_("Kart ve etkileşim"), n_("Fare ayrılınca kartı kapat"), n_("Fare karttan uzaklaşınca kart kapanır"), "kapat fare"),
    e(1, "leave-ms", n_("Kart ve etkileşim"), n_("Kapanma gecikmesi"), n_("Fare ayrıldıktan sonra kartın kapanma süresi"), "gecikme süre kapanma"),
    e(1, "wheel-switches", n_("Kart ve etkileşim"), n_("Tekerlek ve kaydırma etkinlik değiştirir"), n_("Pilin ve kartın üstünde; dokunmatik yüzeyde iki parmakla"), "tekerlek kaydırma fare touchpad"),
    e(1, "keep-user-front", n_("Kart ve etkileşim"), n_("Seçtiğim etkinlik önde kalsın"), n_("Tekerlekle öne aldığın etkinlik yeni bir şey olana dek önde kalır"), "öne al seçim"),
    e(2, "media-on", n_("Medya"), n_("Medyayı göster"), n_("Çalan şarkı pilde; tıklayınca medya kartı"), "müzik şarkı spotify youtube"),
    e(2, "pill-buttons", n_("Pil"), n_("Pil düğmeleri"), n_("Başlık, düğmelere kadar uzar"), "önceki sonraki oynat düğme"),
    e(2, "middle-click", n_("Davranış"), n_("Orta tıkla oynat/durdur"), n_("Medya pilinin üstünde farenin orta tuşu"), "orta tık fare duraklat"),
    e(2, "paused-keep", n_("Davranış"), n_("Duraklatınca pilde kalma süresi"), n_("Sonra pil kaybolur; çalınca geri gelir"), "duraklat kalma süre"),
    e(2, "apps", n_("Uygulamalar"), n_("Uygulamalar"), n_("Pilde gösterilecek medya uygulamaları"), "spotify brave chrome gizle uygulama"),
    e(2, "devices", n_("Ses çıkışı seçicisi"), n_("Ses çıkışı seçicisi"), n_("Medya kartındaki çıkış sayfasında görünen cihazlar"), "hoparlör kulaklık çıkış cihaz ses"),
    e(2, "pill-mute", n_("Pill"), n_("Mute button"), n_("A speaker next to the controls that mutes the playing app"), "mute unmute volume speaker sound button sessiz ses"),
    e(2, "media-pinned", n_("Pil"), n_("Always show"), n_("Even when nothing is playing"), "media always show pinned nothing playing hep görünür sabit boş"),
    e(6, "fullscreen-hide", n_("Tam ekran"), n_("Tam ekranda gizle"), n_("Pil ve kartlar, tam ekran bir uygulama öndeyken gizlenir"), "oyun video sunum gizle"),
    e(6, "exceptions", n_("İstisnalar"), n_("İstisnalar"), n_("Bu uygulamalar tam ekran olsa da Swift Tab görünür kalır"), "istisna uygulama exe"),
    e(7, "log", n_("Dosyalar"), n_("Günlük"), n_("Swift Tab'ın günlük dosyası"), "log kayıt"),
    e(7, "file", n_("Dosyalar"), n_("Ayar dosyası"), n_("settings.json"), "json dosya"),
    e(7, "reset", n_("Sıfırla"), n_("Varsayılanlara dön"), n_("Bütün ayarlar sıfırlanır"), "sıfırla varsayılan reset"),
    e(14, "capture-moment", n_("Ekran görüntüleri"), n_("Ekran görüntüsünü göster"), n_("ShareX ya da Ekran Alıntısı Aracı yeni bir görüntü kaydedince pilde kısa bir an; sürükleyip atabilirsin"), "ekran görüntüsü screenshot sharex alıntı snipping kırpma yakalama"),
    e(14, "transfer-on", n_("Dosya kopyalama"), n_("Kopyalama ilerlemesi"), n_("Explorer kopyalar, taşır ya da silerken pilde yüzde ve kalan süre; penceresi küçültülse de"), "kopyala kopyalama taşı sil dosya ilerleme explorer gezgin yüzde kalan süre copy"),
    e(14, "capture-to-shelf", n_("Ekran görüntüleri"), n_("Ekran görüntülerini rafa ekle"), n_("Yeni görüntüler rafa düşer, sonra sürükleyip atmak için"), "ekran görüntüsü raf shelf sharex screenshot"),
    e(9, "games-downloads", n_("Pilde"), n_("Game downloads"), n_("Progress and speed of Steam and Epic Games downloads in the pill"), "oyun indirme download steam epic ilerleme hız speed progress game"),
    e(9, "games-steam", n_("Mağazalar"), "Steam", n_("Reads the Steam library on this PC"), "steam oyun kütüphane library game"),
    e(9, "games-epic", n_("Mağazalar"), "Epic Games", n_("Reads the Epic Games launcher on this PC"), "epic games oyun kütüphane library launcher game"),
    e(9, "games-playtime", n_("Pilde"), n_("Playtime"), n_("During and after gameplay"), "oyun süre playtime oynama zamanı game time"),
    e(4, "headsets-on", n_("Kulaklıklar"), n_("Diğer kulaklıkları göster"), n_("Bluetooth kulaklıklar, pilleriyle köşe çubuğunda"), "kulaklık bluetooth pil sony airpods jbl"),
    e(8, "idle-clock", n_("Saat ve hava"), n_("Boşta hava ve konum"), n_("Etkinlik yokken köşede sıcaklığı ve şehri gösterir"), "saat tarih hava durumu boşta idle clock"),
    e(8, "clock-12h", n_("Saat ve hava"), n_("12-hour clock"), n_("7:42 PM instead of 19:42"), "12 saat 24 saat am pm öğleden sonra saat biçimi format 12-hour 24-hour clock"),
    e(8, "weather-in-stack", n_("Hava çubuğu"), n_("Hava çubuğu"), n_("Köşede kulaklık pilinin yanında, hep görünür"), "hava durumu çubuk pil yanında her zaman"),
    e(8, "weather-effects", n_("Saat ve hava"), n_("Hava efektleri"), n_("Kar, yağmur, güneş ve yıldızlar havanın arkasında hareket eder"), "efekt animasyon kar yağmur güneş yıldız fırtına sis bulut"),
    e(8, "rain-moment", n_("Saat ve hava"), n_("Yağmur uyarısı"), n_("Şehrinde yağmur ya da kar başlamadan önce pil bir an öne gelir"), "yağmur kar yağış uyarı şemsiye hava"),
    e(8, "weather-alerts", n_("Saat ve hava"), n_("Hava uyarıları"), n_("Fırtına, kuvvetli rüzgâr, aşırı sıcak, don ve kötü hava kalitesi öncesinde pil bir an öne gelir"), "fırtına şimşek gök gürültüsü rüzgar rüzgâr fırtına sıcak sıcaklık don buz kirlilik hava kalitesi aqi uyarı"),
    e(8, "weather-locate", n_("Saat ve hava"), n_("Konumumu kullan"), n_("Şehir seçilmemişse hava bilgisayarın bulunduğu yerin; konum Windows'tan, yer adı OpenStreetMap'ten"), "konum otomatik yer neredeyim gps location"),
    e(8, "weather-city", n_("Saat ve hava"), n_("Hava için şehir"), n_("Sıcaklık ve gökyüzü bu şehrin; veriler Open-Meteo'dan gelir"), "şehir il konum hava durumu sıcaklık derece weather"),
];

/// Lower case without Turkish marks, one character for one (so match positions carry over):
/// İ, I, ı → i; ş → s, ç → c, ğ → g, ö → o, ü → u, â/î/û → a/i/u.
pub fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'İ' | 'I' | 'ı' | 'î' | 'Î' => 'i',
            'Ş' | 'ş' => 's',
            'Ç' | 'ç' => 'c',
            'Ğ' | 'ğ' => 'g',
            'Ö' | 'ö' => 'o',
            'Ü' | 'ü' | 'û' | 'Û' => 'u',
            'Â' | 'â' => 'a',
            c => c.to_lowercase().next().unwrap_or(c),
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    pub page: u8,
    pub row: &'static str,
    pub section: &'static str,
    pub title: &'static str,
    /// "Görünüm › Pil".
    pub path: String,
    pub desc: &'static str,
    /// The matched part of the title, in characters (0, 0 when the match was elsewhere).
    pub mark_start: usize,
    pub mark_len: usize,
}

/// Rows matching `query`: title starting with it first, then title containing it, then
/// description, section, page or keywords; ties keep page order.
pub fn search(query: &str) -> Vec<SearchHit> {
    let q = fold(query.trim());
    if q.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(u8, SearchHit)> = INDEX
        .iter()
        .filter_map(|x| {
            // What the row shows is in the interface's language; its Turkish words find it too.
            let (shown, page) = (tr(x.title), tr(PAGES[x.page as usize]));
            let title = fold(shown);
            let at = title.find(&q).map(|byte| title[..byte].chars().count());
            let others = [tr(x.desc), tr(x.section), page, x.title, x.desc, x.section, PAGES[x.page as usize], x.keywords];
            let score = match at {
                Some(0) => 3,
                Some(_) => 2,
                None if others.iter().any(|t| fold(t).contains(&q)) => 1,
                None => return None,
            };
            let (mark_start, mark_len) = at.map_or((0, 0), |i| (i, q.chars().count()));
            let path = format!("{page} › {}", tr(x.section));
            Some((score, SearchHit { page: x.page, row: x.row, section: tr(x.section), title: shown, path, desc: tr(x.desc), mark_start, mark_len }))
        })
        .collect();
    scored.sort_by_key(|(score, h)| (std::cmp::Reverse(*score), h.page));
    scored.into_iter().map(|(_, h)| h).collect()
}

/// A fullscreen exception as typed: trimmed, ".exe" added. `Ok(None)` for nothing typed;
/// `Err` with the message the window shows.
pub fn exception_name(text: &str, existing: &[String]) -> Result<Option<String>, String> {
    let name = text.trim();
    if name.is_empty() {
        return Ok(None);
    }
    if name.chars().any(|c| matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')) {
        return Err("Invalid name".into());
    }
    let exe = if name.to_lowercase().ends_with(".exe") { name.to_string() } else { format!("{name}.exe") };
    match existing.iter().find(|x| x.eq_ignore_ascii_case(&exe)) {
        Some(have) => Err(format!("{have} is already in the list")),
        None => Ok(Some(exe)),
    }
}

/// The picker's presets as saved: 1–180 minutes, no repeats, ascending, at most six (v1's six
/// when nothing valid is left).
pub fn tidy_presets(v: &[i32]) -> Vec<u32> {
    let mut m: Vec<u32> = v.iter().filter(|&&x| (1..=180).contains(&x)).map(|&x| x as u32).collect();
    m.sort_unstable();
    m.dedup();
    m.truncate(6);
    if m.is_empty() {
        vec![1, 3, 5, 10, 15, 30]
    } else {
        m
    }
}

/// The letters on a crest stand-in: ESPN's code, else the initials of two words, else the first
/// three letters.
pub fn team_code(name: &str, abbr: &str) -> String {
    if !abbr.trim().is_empty() {
        return abbr.trim().to_string();
    }
    let words: Vec<&str> = name.split_whitespace().collect();
    let code: String = if words.len() >= 2 { words.iter().take(2).filter_map(|w| w.chars().next()).collect() } else { name.chars().take(3).collect() };
    code.to_uppercase()
}

/// The highest settings page.
pub const LAST_PAGE: u8 = PAGES.len() as u8 - 1;

/// `--page N` on the settings window's command line: the page to open on.
pub fn page_arg(args: &[String]) -> Option<u8> {
    let i = args.iter().position(|a| a == "--page")?;
    args.get(i + 1)?.parse::<u8>().ok().filter(|p| *p <= LAST_PAGE)
}

/// The letter on a media app's badge: its name's first letter or digit, upper-cased.
pub fn badge_letter(name: &str) -> String {
    name.chars().find(|c| c.is_alphanumeric()).map_or_else(|| "?".to_string(), |c| c.to_uppercase().collect())
}

/// The badge colour of a media app in the settings list (notes §2.1).
pub fn app_badge(app_id: &str) -> [u8; 3] {
    let id = app_id.to_ascii_lowercase();
    let table: [(&str, [u8; 3]); 8] = [
        ("spotify", [0x1D, 0xB9, 0x54]),
        ("youtube", [0xFF, 0x00, 0x33]),
        ("brave", [0xFB, 0x54, 0x2B]),
        ("discord", [0x58, 0x65, 0xF2]),
        ("chrome", [0x42, 0x85, 0xF4]),
        ("msedge", [0x0C, 0x59, 0xA4]),
        ("firefox", [0xFF, 0x71, 0x39]),
        // Firefox's media sessions come with this id.
        ("308046b0af4a39cb", [0xFF, 0x71, 0x39]),
    ];
    table.iter().find(|(k, _)| id.contains(k)).map_or([0x8A, 0x93, 0xB4], |(_, c)| *c)
}

/// A steady, never-dark colour for a team without colours (from its id).
pub fn stand_in_colour(id: &str) -> [u8; 3] {
    let hash = id.bytes().fold(0x811C_9DC5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193));
    let hue = (hash % 360) as f32;
    let (s, v) = (0.55f32, 0.78f32);
    let c = v * s;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = match (hue / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [((r + m) * 255.0) as u8, ((g + m) * 255.0) as u8, ((b + m) * 255.0) as u8]
}

/// The Galaxy Buds page's device list: its labels, what each stands for (`None`: automatic) and
/// the one picked now.
#[derive(Debug, PartialEq)]
pub struct BudsChoices {
    pub labels: Vec<String>,
    pub values: Vec<Option<BudsDevice>>,
    pub current: usize,
}

/// "Otomatik", then the paired candidates (Samsung's service first; a name two of them share
/// gets its address's end), then the chosen pair when it's no longer paired.
pub fn buds_choices(paired: &[Paired], chosen: Option<&BudsDevice>) -> BudsChoices {
    let mut order: Vec<&Paired> = paired.iter().collect();
    order.sort_by_key(|p| !p.samsung);
    let mut labels = vec!["Automatic".to_string()];
    let mut values = vec![None];
    for p in &order {
        let name = if p.name.trim().is_empty() { "Unnamed device" } else { p.name.trim() };
        let shared = order.iter().filter(|o| o.name.trim() == p.name.trim()).count() > 1;
        let end = &p.address[p.address.len().saturating_sub(5)..];
        labels.push(if shared { format!("{name} · {end}") } else { name.to_string() });
        values.push(Some(BudsDevice { address: p.address.clone(), name: p.name.clone() }));
    }
    let at = |c: &BudsDevice| values.iter().position(|v| v.as_ref().is_some_and(|v| v.address.eq_ignore_ascii_case(&c.address)));
    let current = match chosen {
        None => 0,
        Some(c) => match at(c) {
            Some(i) => i,
            None => {
                labels.push(trf("{} · eşleşmemiş", &[&c.name]));
                values.push(Some(c.clone()));
                values.len() - 1
            }
        },
    };
    BudsChoices { labels, values, current }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_speaks_the_interfaces_language() {
        use crate::i18n::{in_language, Language};
        in_language(Language::English, || {
            let hits = search("opacity");
            assert!(hits.iter().any(|h| h.row == "pill-opacity" && h.title == "Background opacity"), "an English word finds the row");
            assert!(search("opaklığı").iter().any(|h| h.row == "pill-opacity"), "and the Turkish still does");
            let h = search("opacity").into_iter().find(|h| h.row == "pill-opacity").unwrap();
            assert_eq!((h.mark_start, h.mark_len), (11, 7), "the mark is on the English title");
            assert!(h.path.starts_with("Appearance › "), "{}", h.path);
        });
    }

    #[test]
    fn buds_choices_start_with_automatic_and_keep_a_lost_pick() {
        let p = |address: &str, name: &str, samsung| Paired { address: address.into(), name: name.into(), samsung };
        let all = [p("02:00:00:00:00:03", "HC-05", false), p("02:00:00:00:00:01", "Galaxy Buds2", true), p("02:00:00:00:00:02", "Galaxy Buds2", true), p("02:00:00:00:00:04", "", false)];
        let c = buds_choices(&all, None);
        assert_eq!(c.labels, ["Automatic", "Galaxy Buds2 · 00:01", "Galaxy Buds2 · 00:02", "HC-05", "Unnamed device"]);
        assert_eq!((c.current, &c.values[0]), (0, &None));
        assert_eq!(c.values[3], Some(BudsDevice { address: "02:00:00:00:00:03".into(), name: "HC-05".into() }));
        let chosen = BudsDevice { address: "02:00:00:00:00:02".into(), name: "Galaxy Buds2".into() };
        assert_eq!(buds_choices(&all, Some(&chosen)).current, 2);
        let lost = BudsDevice { address: "02:00:00:00:00:09".into(), name: "Eski Buds".into() };
        let c = buds_choices(&all, Some(&lost));
        assert_eq!((c.labels.last().map(String::as_str), c.current, c.values.last()), (Some("Eski Buds · not paired"), 5, Some(&Some(lost.clone()))));
    }

    #[test]
    fn folding_ignores_turkish_letters_and_case() {
        assert_eq!(fold("OPAKLIK"), "opaklik");
        assert_eq!(fold("İstisna Şerit Çıkış Ğ Ö Ü"), "istisna serit cikis g o u");
        assert_eq!(fold("bulanık"), fold("BULANIK"));
    }

    #[test]
    fn search_finds_rows_by_title_description_and_keywords() {
        let hits = search("opacity");
        assert_eq!(hits[0].title, "Background opacity");
        assert_eq!(hits[0].page, 0);
        assert_eq!((hits[0].mark_start, hits[0].mark_len), (11, 7));
        // A keyword only ("blur") still finds the row.
        assert!(search("blur").iter().any(|h| h.title == "Blur"));
        // Accent-insensitive / search: "clipboard" finds "Clipboard history".
        assert!(search("clipboard").iter().any(|h| h.title == "Clipboard history"));
        assert!(search("").is_empty());
        assert!(search("zzzz").is_empty());
    }

    #[test]
    fn search_finds_the_games() {
        let rows = |q: &str, page: u8| search(q).into_iter().filter(|h| h.page == page).map(|h| h.row).collect::<Vec<_>>();
        assert_eq!(tr(PAGES[9]), "Games");
        assert!(rows("steam", 9).contains(&"games-steam"), "{:?}", rows("steam", 9));
        assert!(rows("epic", 9).contains(&"games-epic"), "{:?}", rows("epic", 9));
        assert!(rows("download", 9).contains(&"games-downloads"), "{:?}", rows("download", 9));
        assert_eq!(LAST_PAGE, 14);
    }

    #[test]
    fn search_ranks_title_starts_first() {
        let hits = search("card");
        assert!(hits[0].title.to_lowercase().starts_with("card"), "{:?}", hits[0].title);
        assert_eq!(hits[0].path, format!("{} › {}", tr(PAGES[hits[0].page as usize]), hits[0].section));
    }

    #[test]
    fn search_finds_the_clock_and_weather() {
        let rows = |q: &str| search(q).into_iter().filter(|h| h.page == 8).map(|h| h.row).collect::<Vec<_>>();
        assert!(rows("clock").contains(&"idle-clock"), "{:?}", rows("clock"));
        assert!(rows("weather").contains(&"idle-clock"), "{:?}", rows("weather"));
        assert!(rows("city").contains(&"weather-city"), "{:?}", rows("city"));
        assert!(rows("rain").contains(&"rain-moment"), "{:?}", rows("rain"));
        assert_eq!(search("city")[0].path, "Weather › Clock and weather");
        assert!(rows("effects").contains(&"weather-effects"), "{:?}", rows("effects"));
        for q in ["storm", "wind", "frost", "air quality", "alert"] {
            assert!(rows(q).contains(&"weather-alerts"), "{q}: {:?}", rows(q));
        }
        assert!(rows("snow").contains(&"weather-effects"), "{:?}", rows("snow"));
    }

    #[test]
    fn exception_names_are_checked_and_get_exe() {
        let have = vec!["vlc.exe".to_string()];
        assert_eq!(exception_name("  mpv ", &have), Ok(Some("mpv.exe".into())));
        assert_eq!(exception_name("Game.EXE", &have), Ok(Some("Game.EXE".into())));
        assert_eq!(exception_name("   ", &have), Ok(None));
        assert_eq!(exception_name("VLC", &have), Err("vlc.exe is already in the list".into()));
        assert_eq!(exception_name("a/b.exe", &have), Err("Invalid name".into()));
    }

    #[test]
    fn presets_are_sorted_unique_in_range_and_at_most_six() {
        assert_eq!(tidy_presets(&[30, 5, 5, 0, 200, 1, 10, 15, 3, 45]), vec![1, 3, 5, 10, 15, 30]);
        assert_eq!(tidy_presets(&[]), vec![1, 3, 5, 10, 15, 30]);
        assert_eq!(tidy_presets(&[90, 7]), vec![7, 90]);
    }

    #[test]
    fn page_arg_parses() {
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(page_arg(&args(&["swifttab-settings.exe", "--page", "5"])), Some(5));
        assert_eq!(page_arg(&args(&["swifttab-settings.exe"])), None);
        assert_eq!(page_arg(&args(&["x", "--page", &(LAST_PAGE + 1).to_string()])), None, "no such page");
        assert_eq!(page_arg(&args(&["x", "--page", "ekin"])), None);
        assert_eq!(page_arg(&args(&["x", "--page"])), None);
    }

    #[test]
    fn badge_letter_is_the_first_letter_or_digit() {
        assert_eq!(badge_letter("Spotify"), "S");
        assert_eq!(badge_letter("youTube Music"), "Y");
        assert_eq!(badge_letter("  (beta) Brave"), "B");
        assert_eq!(badge_letter("7-Zip"), "7");
        assert_eq!(badge_letter("şarkıcı"), "Ş");
        assert_eq!(badge_letter(""), "?");
    }

    #[test]
    fn app_badges_by_app() {
        assert_eq!(app_badge("Spotify.exe"), [0x1D, 0xB9, 0x54]);
        assert_eq!(app_badge("com.github.th-ch.youtube-music"), [0xFF, 0x00, 0x33]);
        assert_eq!(app_badge("Brave"), [0xFB, 0x54, 0x2B]);
        assert_eq!(app_badge("MSEdge"), [0x0C, 0x59, 0xA4]);
        assert_eq!(app_badge("308046B0AF4A39CB"), [0xFF, 0x71, 0x39]);
        assert_eq!(app_badge("vlc.exe"), [0x8A, 0x93, 0xB4]);
    }

    #[test]
    fn team_stand_in_code_and_colours() {
        assert_eq!(team_code("Real Madrid", ""), "RM");
        assert_eq!(team_code("Liverpool", "LIV"), "LIV");
        assert_eq!(team_code("Beşiktaş", ""), "BEŞ");
        // No colours from ESPN: a steady colour from the id, never black.
        let (a, b) = (stand_in_colour("364"), stand_in_colour("364"));
        assert_eq!(a, b);
        assert_ne!(stand_in_colour("364"), stand_in_colour("102"));
        assert!(a.iter().any(|&c| c > 60));
    }
}
