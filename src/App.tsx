import React, { useEffect, useState } from 'react';
import {
  Download,
  Volume2,
  Headphones,
  Pin,
  Trash2,
  FolderArchive,
  Share2,
  FileText,
  Minimize2,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Shuffle,
  Repeat,
  Check,
  Cloud,
  CloudRain,
  Sun,
  Timer,
  Code2,
  Settings,
  Eye,
  ShieldCheck,
  Sparkles,
  Copy,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';

interface ClipTile {
  id: number;
  resolution: string;
  gradient: string;
  label?: string;
}

interface ShelfTileItem {
  id: number;
  name: string;
  fullName: string;
  kind: 'pdf' | 'doc' | 'zip' | 'code' | 'img';
  pinned: boolean;
}

interface TrackItem {
  title: string;
  artist: string;
  source: string;
  sourceColor: string;
  output: string;
  durationSec: number;
  positionSec: number;
  tintHex: string;
  artGradient: string;
  hasArt: boolean;
}

const TRACKS: TrackItem[] = [
  {
    title: 'GEM DELÍRIO (Slowed)',
    artist: 'ZXVRF',
    source: 'Spotify',
    sourceColor: '#1ED760',
    output: 'Galaxy Buds2 (85%)',
    durationSec: 171,
    positionSec: 84,
    tintHex: 'rgba(138, 36, 48, 0.24)',
    artGradient:
      'radial-gradient(circle at 60% 45%, #ff6b4a 0%, #c81d25 35%, #3d0814 70%, #0d0204 100%)',
    hasArt: true,
  },
  {
    title: 'After Dark',
    artist: 'Mr.Kitty',
    source: 'Spotify',
    sourceColor: '#1ED760',
    output: 'Galaxy Buds2 (85%)',
    durationSec: 259,
    positionSec: 112,
    tintHex: 'rgba(46, 92, 168, 0.24)',
    artGradient:
      'radial-gradient(circle at 40% 35%, #60cdff 0%, #1e3a8a 48%, #0f172a 85%, #050811 100%)',
    hasArt: true,
  },
  {
    title: 'Untitled Session.wav',
    artist: 'Local Audio Stream',
    source: 'System Media',
    sourceColor: '#60CDFF',
    output: 'Speakers (Realtek Audio)',
    durationSec: 198,
    positionSec: 45,
    tintHex: 'transparent',
    artGradient: 'linear-gradient(135deg, rgba(255,255,255,0.08), rgba(255,255,255,0.03))',
    hasArt: false,
  },
];

const INITIAL_CLIPS: ClipTile[] = [
  {
    id: 1,
    resolution: '1920×1080',
    gradient: 'linear-gradient(135deg, #004e92, #000428)',
  },
  {
    id: 2,
    resolution: '1484×803',
    gradient: 'linear-gradient(135deg, #1a2a6c, #b21f1f, #fdbb2d)',
  },
  {
    id: 3,
    resolution: '2560×1440',
    gradient: 'linear-gradient(135deg, #134e5e, #71b280)',
  },
  {
    id: 4,
    resolution: '800×600',
    gradient: 'linear-gradient(135deg, #3a1c71, #d76d77, #ffaf7b)',
  },
  {
    id: 5,
    resolution: 'github.com',
    gradient: 'linear-gradient(135deg, #232526, #414345)',
    label: 'github.com/SeNsXD/Syro',
  },
  {
    id: 6,
    resolution: '1280×720',
    gradient: 'linear-gradient(135deg, #1f4037, #99f2c8)',
  },
];

const INITIAL_SHELF: ShelfTileItem[] = [
  {
    id: 1,
    name: '264683_1294729_rapor.pdf',
    fullName: '264683_1294729_rapor.pdf',
    kind: 'pdf',
    pinned: true,
  },
  {
    id: 2,
    name: 'Notlar_2026_Final.txt',
    fullName: 'Notlar_2026_Final.txt',
    kind: 'doc',
    pinned: false,
  },
  {
    id: 3,
    name: 'Proje_Sunum_V2.pptx',
    fullName: 'Proje_Sunum_V2.pptx',
    kind: 'doc',
    pinned: false,
  },
  {
    id: 4,
    name: 'syro_now_bar_win11.zip',
    fullName: 'syro_now_bar_win11.zip',
    kind: 'zip',
    pinned: false,
  },
  {
    id: 5,
    name: 'card_renderer_d2d.rs',
    fullName: 'card_renderer_d2d.rs',
    kind: 'code',
    pinned: false,
  },
];

const CHANGED_FILES = [
  {
    path: 'src/platform/os.rs',
    area: 'OS & Accessibility Queries',
    summary:
      'Added system_accent() via UISettings (UIColorType::AccentLight2 / AccentDark1), transparency_enabled(), battery_saver(), high_contrast(), animations_enabled(), and solid_fallback().',
  },
  {
    path: 'src/platform/frame.rs',
    area: 'Windows 11 Mica Backdrop',
    summary:
      'Applied DWMWA_SYSTEMBACKDROP_TYPE = DWMSBT_MAINWINDOW (2) for Settings & Setup windows (or DWMSBT_NONE in solid fallback) with #202020 / #F3F3F3 frame colors.',
  },
  {
    path: 'src/platform/windows.rs',
    area: 'Pill & Card Host Backdrop',
    summary:
      'Enabled DWMWA_USE_HOSTBACKDROPBRUSH on PillWindow::create alongside CardWindow::create for single window-level Acrylic.',
  },
  {
    path: 'src/render/icons.rs',
    area: 'Segoe Fluent Icons',
    summary:
      'Added Icon::Pin (\\u{E718}), Icon::Cloud (\\u{E753}), Icon::Rain (\\u{E9C4}), Icon::Sun (\\u{E706}), Icon::Snow (\\u{E9C8}), and weather_icon_for_symbol() so weather uses Fluent glyphs instead of emoji.',
  },
  {
    path: 'src/render/gfx.rs',
    area: 'Typography & Cover Art',
    summary:
      'Switched DirectWrite font loading to system Segoe UI Variable Text / Display (fallback Segoe UI); added sec_text(), neutral_icon() (#D6D6D6), and image_cover_rounded().',
  },
  {
    path: 'src/render/color.rs',
    area: '24% Dynamic Surface Tint',
    summary:
      'Added tint_mix_24() to blend 24% album art dominant color or sky tint into #202020.',
  },
  {
    path: 'src/render/motion.rs',
    area: 'Reduced Motion Support',
    summary:
      'Updated speed_factor() to skip animations when Windows client-area animations (SPI_GETCLIENTAREAANIMATION) are disabled.',
  },
  {
    path: 'src/render/morph.rs',
    area: 'Window Acrylic & Shadow Chrome',
    summary:
      'Updated shadow to 32px blur / 12px Y-offset / 28% opacity, #202020CC Acrylic base (#202020 solid fallback), 1px #FFFFFF17 outer border, 8px corner radius, and cubic-bezier(0, 0, 0, 1) transitions.',
  },
  {
    path: 'src/render/pill.rs',
    area: '44px Now Bar Overlay Rows',
    summary:
      'Restyled all bar states to 44px height, 8px radius, 3×16px left accent pill + white 9% active fill, 1px vertical dividers, 28×28px 4px-radius cover art, 4-bar system accent equalizer, and #D6D6D6 weather/earbuds chips.',
  },
  {
    path: 'src/activities/clipboard_stash.rs',
    area: 'Clipboard 3-Column Grid Metrics',
    summary:
      'Updated rows() and card_height_dip() to a 3-column 4:3 thumbnail grid layout.',
  },
  {
    path: 'src/activities/shelf.rs',
    area: 'Shelf 3-Column Grid & Footer Band',
    summary:
      'Updated rows(), card_height_dip(), and FOOTER_BAND_DIP (44 DIP) for 3-column square file tiles plus the bottom footer band.',
  },
  {
    path: 'src/render/card.rs',
    area: 'Clipboard, Shelf, Media & Weather Panels',
    summary:
      'Implemented 8px panel radius, 24% track/sky tinting, 3-column Clipboard & Shelf grids, 2-line wrapped filenames with pinned sorting, 96px Media art with 20px thumb slider & 48px glowing accent Play button, and Fluent Weather forecast.',
  },
  {
    path: 'ui/theme.slint',
    area: 'Slint Fluent Theme Tokens',
    summary:
      'Updated Slint theme palette to Windows 11 Mica/Acrylic (#202020 ground, #FFFFFF0D cards, #FFFFFFA6 secondary text, #60CDFF system accent).',
  },
  {
    path: 'ui/kit.slint',
    area: 'Slint Shared Controls',
    summary:
      'Updated LRow, Tile, and SideItem radii (8px tiles, 4px rows/buttons) and cubic-bezier(0, 0, 0, 1) transitions.',
  },
  {
    path: 'ui/pills.slint',
    area: 'Slint Settings Preview Bar Rows',
    summary:
      'Updated preview Now Bar rows to 44px height, 8px radius, #202020CC fill, 1px border, and 3×16px left accent indicator.',
  },
  {
    path: 'ui/settings.slint',
    area: 'Slint Settings Window',
    summary:
      'Switched default-font-family to Segoe UI Variable Text (Theme.text-face) and updated capsule/menu radii.',
  },
  {
    path: 'setup/ui/setup.slint',
    area: 'Slint Installer Window',
    summary:
      'Updated SetupWindow to Segoe UI Variable Text, 44px stage row, 8px stage radius, and 4px button radius.',
  },
  {
    path: 'i18n/en/LC_MESSAGES/nowbar-ui.po',
    area: 'English Localization Fix',
    summary:
      'Added missing English translations for "{} öğe" -> "{} items" (Clipboard count) and "Şimdi" -> "Now" (Weather header).',
  },
  {
    path: 'src/app.rs',
    area: 'Idle Weather Location & Settings Fallback',
    summary:
      'Appended current city name ("24° · Giridih") to idle weather bar when earbuds are disconnected, and added automatic cargo build fallback for nowbar-settings.exe.',
  },
  {
    path: 'src/main.rs',
    area: 'Second Instance Settings Launch',
    summary:
      'Updated second-instance launch to call open_settings_at(None) with automatic cargo fallback.',
  },
  {
    path: 'Cargo.toml',
    area: 'Default Binary Manifest Key',
    summary:
      'Added default-run = "nowbar" so cargo run --release launches the main Now Bar binary without needing --bin.',
  },
  {
    path: 'src/activities/media_worker.rs',
    area: 'Repeat-One (2nd Click) State',
    summary:
      'Mapped AutoRepeatMode::List to Some(false) and AutoRepeatMode::Track (2nd click) to Some(true) so Repeat-One is distinguished.',
  },
  {
    path: 'src/platform/tray.rs',
    area: 'Right-Click Context Menus & Streamlined Tray',
    summary:
      'Added show_clear_menu() for Clipboard/Shelf right-click Clear All and show_media_menu() for Media right-click Open <App>, and removed unused tray items.',
  },
  {
    path: 'src/platform/fullscreen.rs',
    area: 'Focus Playing Media App',
    summary:
      'Added focus_media_app() to bring the playing media application window to the foreground (or launch via ShellExecuteW) when clicked from the Media right-click menu.',
  },
  {
    path: 'ui/pages.slint',
    area: 'Redesigned Windows 11 Settings Pages & Mockups',
    summary:
      'Added live Win11TaskbarMockup showing corner Weather+Earbuds bar + front pill, updated Pill Sizes (Compact 78%, Normal 88%, Large 100%), and removed lyrics & unused settings.',
  },
  {
    path: 'src/settings_ui.rs',
    area: 'Streamlined Settings Search Index',
    summary:
      'Removed lyrics, wallpaper accent, and unused entries from the Settings search index.',
  },
];

function formatTime(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s < 10 ? '0' : ''}${s}`;
}

export default function App() {
  const [activeBarRow, setActiveBarRow] = useState<'clipboard' | 'media' | 'shelf'>('clipboard');
  const [solidFallback, setSolidFallback] = useState(false);
  const [accentColor, setAccentColor] = useState('#60CDFF');
  const [tab, setTab] = useState<'preview' | 'files'>('preview');

  // File contents map for viewing/copying/downloading individual files
  const [fileContents, setFileContents] = useState<Record<string, string>>({});
  const [expandedFile, setExpandedFile] = useState<string | null>(null);
  const [copiedPath, setCopiedPath] = useState<string | null>(null);
  const [downloading, setDownloading] = useState<string | null>(null);

  useEffect(() => {
    fetch('/changed-files.json')
      .then((r) => (r.ok ? r.json() : {}))
      .then((data) => setFileContents(data))
      .catch(() => {});
  }, []);

  const handleDownloadArchive = async (url: string, filename: string) => {
    try {
      setDownloading(filename);
      const res = await fetch(url);
      const blob = await res.blob();
      const blobUrl = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = blobUrl;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(blobUrl), 5000);
    } finally {
      setDownloading(null);
    }
  };

  const handleDownloadSingleFile = (relPath: string) => {
    const content = fileContents[relPath];
    if (!content) return;
    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    const blobUrl = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = blobUrl;
    a.download = relPath.split('/').pop() || relPath;
    document.body.appendChild(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(blobUrl), 3000);
  };

  const handleCopyFile = async (relPath: string) => {
    const content = fileContents[relPath];
    if (!content) return;
    await navigator.clipboard.writeText(content);
    setCopiedPath(relPath);
    setTimeout(() => setCopiedPath(null), 2000);
  };

  // Clipboard state
  const [clips, setClips] = useState<ClipTile[]>(INITIAL_CLIPS);
  const [selectedClipId, setSelectedClipId] = useState<number>(1);

  // Shelf state
  const [shelfItems, setShelfItems] = useState<ShelfTileItem[]>(INITIAL_SHELF);
  const [shelfPinned, setShelfPinned] = useState<boolean>(true);
  const [shelfToast, setShelfToast] = useState<string | null>(null);

  // Media state: repeatMode 0 = Off, 1 = Repeat List (1st click), 2 = Repeat Track "1" (2nd click)
  const [trackIdx, setTrackIdx] = useState(0);
  const [playing, setPlaying] = useState(true);
  const [shuffleOn, setShuffleOn] = useState(true);
  const [repeatMode, setRepeatMode] = useState<0 | 1 | 2>(2);
  const [mediaPinned, setMediaPinned] = useState(false);
  const [showOutputPicker, setShowOutputPicker] = useState(false);
  const [selectedOutput, setSelectedOutput] = useState('Galaxy Buds2 (85%)');
  const [volumePct, setVolumePct] = useState(85);
  const [positionSec, setPositionSec] = useState(TRACKS[0].positionSec);
  const [pillSizeIdx, setPillSizeIdx] = useState<0 | 1 | 2>(1);
  const [contextMenu, setContextMenu] = useState<{
    kind: 'clipboard' | 'shelf' | 'media';
    label: string;
  } | null>(null);

  // Weather state
  const [weatherMode, setWeatherMode] = useState<'overcast' | 'rain'>('overcast');
  const [selectedHourIdx, setSelectedHourIdx] = useState(0);

  const currentTrack = TRACKS[trackIdx];
  const sortedShelf = [...shelfItems].sort((a, b) => Number(b.pinned) - Number(a.pinned));

  const triggerShelfTool = (label: string) => {
    setShelfToast(label);
    setTimeout(() => setShelfToast(null), 2200);
  };

  const panelBg = solidFallback ? '#202020' : 'rgba(32, 32, 32, 0.80)';
  const outerBorder = solidFallback ? 'rgba(255, 255, 255, 0.35)' : 'rgba(255, 255, 255, 0.09)';
  const secondaryText = solidFallback ? 'rgba(255, 255, 255, 0.82)' : 'rgba(255, 255, 255, 0.65)';
  const backdropBlur = solidFallback ? 'none' : 'blur(40px) saturate(150%)';

  return (
    <div
      className="min-h-screen text-white selection:bg-[#60CDFF]/30"
      style={{
        fontFamily: '"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif',
        backgroundImage: `
          radial-gradient(circle at 18% 28%, rgba(28, 65, 115, 0.55) 0%, transparent 50%),
          radial-gradient(circle at 82% 72%, rgba(18, 45, 90, 0.6) 0%, transparent 50%),
          linear-gradient(135deg, #0a1526 0%, #122238 50%, #070d17 100%)
        `,
        backgroundAttachment: 'fixed',
      }}
    >
      {/* Top Toolbar */}
      <header
        className="sticky top-0 z-30 border-b px-6 py-3.5"
        style={{
          background: solidFallback ? '#202020' : 'rgba(24, 24, 24, 0.82)',
          borderColor: outerBorder,
          backdropFilter: backdropBlur,
        }}
      >
        <div className="mx-auto flex max-w-[1240px] flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div
              className="flex h-8 w-8 items-center justify-center rounded-[4px] border border-white/10"
              style={{ background: 'rgba(255,255,255,0.06)' }}
            >
              <div
                className="h-4 w-1 rounded-full"
                style={{ backgroundColor: accentColor }}
              />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h1 className="text-[15px] font-semibold tracking-[-0.01em] text-white">
                  Syro Now Bar
                </h1>
                <span className="rounded-[4px] border border-white/10 bg-white/[0.06] px-2 py-0.5 text-[11px] text-white/75">
                  Windows 11 Fluent Acrylic &amp; Mica
                </span>
              </div>
              <p className="text-[12px]" style={{ color: secondaryText }}>
                Direct2D + Windows.UI.Composition + Win32 DWM + Slint UI Restyle
              </p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            {/* System Accent Switcher */}
            <div className="flex items-center gap-1.5 rounded-[4px] border border-white/10 bg-white/[0.04] px-2.5 py-1">
              <span className="text-[11px]" style={{ color: secondaryText }}>
                Accent:
              </span>
              {[
                { label: 'Win11 Blue', hex: '#60CDFF' },
                { label: 'Mint', hex: '#6CCB5F' },
                { label: 'Lilac', hex: '#C399FF' },
                { label: 'Gold', hex: '#FCE100' },
              ].map((sw) => (
                <button
                  key={sw.hex}
                  onClick={() => setAccentColor(sw.hex)}
                  title={sw.label}
                  aria-label={`Set system accent to ${sw.label}`}
                  className="h-4 w-4 rounded-[3px] transition-transform active:scale-95"
                  style={{
                    backgroundColor: sw.hex,
                    outline: accentColor === sw.hex ? '1.5px solid #fff' : 'none',
                    outlineOffset: '1px',
                  }}
                />
              ))}
            </div>

            {/* Fallback Toggle */}
            <button
              onClick={() => setSolidFallback((v) => !v)}
              className="flex items-center gap-1.5 rounded-[4px] border px-2.5 py-1.5 text-[12px] transition-all duration-150 active:scale-[0.98]"
              style={{
                borderColor: solidFallback ? accentColor : 'rgba(255,255,255,0.09)',
                background: solidFallback ? 'rgba(255,255,255,0.09)' : 'rgba(255,255,255,0.05)',
                color: '#fff',
              }}
            >
              <ShieldCheck className="h-3.5 w-3.5" style={{ color: '#D6D6D6' }} />
              <span>{solidFallback ? 'Solid #202020' : 'Acrylic ON'}</span>
            </button>

            {/* View Switcher */}
            <div className="flex rounded-[4px] border border-white/10 bg-white/[0.04] p-0.5">
              <button
                onClick={() => setTab('preview')}
                className="flex items-center gap-1.5 rounded-[3px] px-2.5 py-1 text-[12px] font-medium transition-colors"
                style={{
                  background: tab === 'preview' ? 'rgba(255,255,255,0.09)' : 'transparent',
                  color: tab === 'preview' ? '#fff' : secondaryText,
                }}
              >
                <Eye className="h-3.5 w-3.5" />
                Live Preview
              </button>
              <button
                onClick={() => setTab('files')}
                className="flex items-center gap-1.5 rounded-[3px] px-2.5 py-1 text-[12px] font-medium transition-colors"
                style={{
                  background: tab === 'files' ? 'rgba(255,255,255,0.09)' : 'transparent',
                  color: tab === 'files' ? '#fff' : secondaryText,
                }}
              >
                <Code2 className="h-3.5 w-3.5" />
                Changed Files ({CHANGED_FILES.length})
              </button>
            </div>

            {/* Download Changed Files Only (Fast Patch Zip) */}
            <button
              onClick={() =>
                handleDownloadArchive(
                  '/syro-win11-changed-files.zip',
                  'syro-win11-changed-files.zip'
                )
              }
              className="flex items-center gap-1.5 rounded-[4px] border border-white/15 bg-white/[0.08] px-3 py-1.5 text-[12px] font-semibold text-white transition-all duration-150 hover:bg-white/[0.12] active:scale-[0.98]"
            >
              <Download className="h-3.5 w-3.5" />
              {downloading === 'syro-win11-changed-files.zip'
                ? 'Downloading…'
                : 'Changed Files (.zip)'}
            </button>

            {/* Download Full Updated Rust Project */}
            <button
              onClick={() =>
                handleDownloadArchive(
                  '/syro-now-bar-win11.zip',
                  'syro-now-bar-win11.zip'
                )
              }
              className="flex items-center gap-1.5 rounded-[4px] px-3 py-1.5 text-[12px] font-semibold text-black transition-all duration-150 hover:brightness-110 active:scale-[0.98]"
              style={{ backgroundColor: accentColor }}
            >
              <Download className="h-3.5 w-3.5 stroke-[2.2]" />
              {downloading === 'syro-now-bar-win11.zip'
                ? 'Downloading…'
                : 'Full Project (.zip)'}
            </button>
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-[1240px] px-6 py-8">
        {tab === 'files' ? (
          <div
            className="rounded-[8px] border p-6"
            style={{
              background: panelBg,
              borderColor: outerBorder,
              backdropFilter: backdropBlur,
              boxShadow: '0 12px 32px rgba(0, 0, 0, 0.28), 0 2px 6px rgba(0, 0, 0, 0.18)',
            }}
          >
            <div className="mb-5 flex flex-wrap items-center justify-between gap-4 border-b border-white/[0.07] pb-4">
              <div>
                <h2 className="text-[18px] font-semibold text-white">
                  Modified Rust &amp; Slint Source Files ({CHANGED_FILES.length})
                </h2>
                <p className="mt-0.5 text-[13px]" style={{ color: secondaryText }}>
                  Download the ZIP archive or view, copy, and download individual files below.
                </p>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <button
                  onClick={() =>
                    handleDownloadArchive(
                      '/syro-win11-changed-files.zip',
                      'syro-win11-changed-files.zip'
                    )
                  }
                  className="flex items-center gap-2 rounded-[4px] border border-white/15 bg-white/[0.08] px-3.5 py-1.5 text-[12px] font-semibold text-white hover:bg-white/[0.12] active:scale-[0.98]"
                >
                  <Download className="h-3.5 w-3.5" />
                  Download 17 Changed Files (.zip)
                </button>
                <button
                  onClick={() =>
                    handleDownloadArchive(
                      '/syro-now-bar-win11.zip',
                      'syro-now-bar-win11.zip'
                    )
                  }
                  className="flex items-center gap-2 rounded-[4px] px-3.5 py-1.5 text-[12px] font-semibold text-black active:scale-[0.98]"
                  style={{ backgroundColor: accentColor }}
                >
                  <Download className="h-3.5 w-3.5" />
                  Download Full Project (.zip)
                </button>
              </div>
            </div>

            <div className="grid gap-2.5">
              {CHANGED_FILES.map((item) => {
                const isOpen = expandedFile === item.path;
                const code = fileContents[item.path];
                return (
                  <div
                    key={item.path}
                    className="overflow-hidden rounded-[4px] border"
                    style={{
                      background: 'rgba(255, 255, 255, 0.05)',
                      borderColor: 'rgba(255, 255, 255, 0.07)',
                    }}
                  >
                    <div className="flex flex-col justify-between gap-3 px-4 py-3 sm:flex-row sm:items-center">
                      <div className="min-w-[240px]">
                        <code
                          className="text-[13px] font-semibold"
                          style={{ color: accentColor }}
                        >
                          {item.path}
                        </code>
                        <div className="text-[11.5px]" style={{ color: secondaryText }}>
                          {item.area}
                        </div>
                      </div>
                      <p className="flex-1 text-[12.5px] text-white/90 sm:px-3">
                        {item.summary}
                      </p>
                      <div className="flex shrink-0 items-center gap-1.5">
                        <button
                          onClick={() => handleCopyFile(item.path)}
                          className="flex items-center gap-1 rounded-[4px] bg-white/[0.06] px-2.5 py-1 text-[11.5px] text-white hover:bg-white/[0.10]"
                        >
                          {copiedPath === item.path ? (
                            <>
                              <Check className="h-3 w-3 text-[#6CCB5F]" />
                              <span>Copied</span>
                            </>
                          ) : (
                            <>
                              <Copy className="h-3 w-3 text-[#D6D6D6]" />
                              <span>Copy</span>
                            </>
                          )}
                        </button>
                        <button
                          onClick={() => handleDownloadSingleFile(item.path)}
                          className="flex items-center gap-1 rounded-[4px] bg-white/[0.06] px-2.5 py-1 text-[11.5px] text-white hover:bg-white/[0.10]"
                        >
                          <Download className="h-3 w-3 text-[#D6D6D6]" />
                          <span>File</span>
                        </button>
                        <button
                          onClick={() =>
                            setExpandedFile(isOpen ? null : item.path)
                          }
                          className="flex items-center gap-1 rounded-[4px] bg-white/[0.06] px-2.5 py-1 text-[11.5px] text-white hover:bg-white/[0.10]"
                        >
                          {isOpen ? (
                            <>
                              <ChevronUp className="h-3.5 w-3.5" />
                              <span>Hide</span>
                            </>
                          ) : (
                            <>
                              <ChevronDown className="h-3.5 w-3.5" />
                              <span>View</span>
                            </>
                          )}
                        </button>
                      </div>
                    </div>
                    {isOpen && code && (
                      <div className="border-t border-white/[0.07] bg-black/40 p-3">
                        <pre className="max-h-[380px] overflow-auto text-[11.5px] leading-relaxed text-white/90">
                          <code>{code}</code>
                        </pre>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        ) : (
          <div className="space-y-9">
            {/* Section 1: Unified Now Bar Companion Bar */}
            <section className="space-y-3">
              <div className="flex flex-wrap items-center justify-between gap-3 border-b border-white/[0.08] pb-1.5">
                <span className="text-[12px] font-semibold uppercase tracking-[0.06em] text-white/60">
                  Unified Windows 11 Taskbar Companion Bar · Corner Status + Front Activity (Right-Click for Context Menu)
                </span>
                {contextMenu && (
                  <div className="flex items-center gap-2 rounded-[6px] border border-white/15 bg-[#202020] px-3 py-1 text-[12px] shadow-lg">
                    <span className="text-white/65">Context Menu:</span>
                    <button
                      onClick={() => {
                        if (contextMenu.kind === 'clipboard') setClips([]);
                        if (contextMenu.kind === 'shelf') setShelfItems([]);
                        if (contextMenu.kind === 'media')
                          triggerShelfTool(`Focused ${currentTrack.source}`);
                        setContextMenu(null);
                      }}
                      className="rounded-[4px] bg-white/[0.08] px-2 py-0.5 font-semibold text-white hover:bg-white/[0.14]"
                    >
                      {contextMenu.label}
                    </button>
                  </div>
                )}
              </div>

              {/* The Unified Bar (One cohesive Fluent container, no fragmented double boxes) */}
              <div className="flex flex-wrap items-center gap-4">
                <div
                  className="flex h-[44px] items-center rounded-[8px] border p-1 transition-all duration-150"
                  style={{
                    background: panelBg,
                    borderColor: outerBorder,
                    backdropFilter: backdropBlur,
                    boxShadow:
                      '0 12px 32px rgba(0, 0, 0, 0.28), 0 2px 6px rgba(0, 0, 0, 0.18)',
                  }}
                >
                  {/* Left: Corner Weather & Earbuds Status */}
                  <div className="flex items-center px-2.5">
                    <div className="flex items-center gap-1.5 pr-2.5 text-[12.5px] font-semibold tabular-nums">
                      <Cloud className="h-4 w-4" style={{ color: accentColor }} />
                      <span>24°</span>
                    </div>
                    <div className="h-[18px] w-[1px] bg-white/[0.08]" />
                    <div className="flex items-center gap-1.5 pl-2.5 text-[12.5px] font-semibold tabular-nums">
                      <Headphones className="h-3.5 w-3.5 text-[#D6D6D6]" />
                      <span>85%</span>
                    </div>
                  </div>

                  {/* Hairline Divider between Corner Status and Front Module */}
                  <div className="mx-1 h-[24px] w-[1px] bg-white/[0.08]" />

                  {/* Front Activity Module: Tabbed between Clipboard / Media / Shelf */}
                  {activeBarRow === 'clipboard' ? (
                    <div
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setContextMenu({ kind: 'clipboard', label: 'Clear All' });
                      }}
                      className="relative flex h-[36px] cursor-pointer items-center gap-2.5 rounded-[6px] bg-white/[0.09] pl-3.5 pr-3 transition-colors"
                    >
                      <span
                        className="absolute left-1 h-[14px] w-[3px] rounded-[1.5px]"
                        style={{ backgroundColor: accentColor }}
                      />
                      <div
                        className="h-[22px] w-[32px] shrink-0 rounded-[4px] border border-white/10"
                        style={{
                          background:
                            clips[0]?.gradient ||
                            'linear-gradient(135deg, #004e92, #000428)',
                        }}
                      />
                      <span className="text-[13px] font-semibold text-white">
                        Clipboard
                      </span>
                      <span
                        className="text-[12.5px] tabular-nums"
                        style={{ color: secondaryText }}
                      >
                        {clips.length} items
                      </span>
                    </div>
                  ) : activeBarRow === 'media' ? (
                    <div
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setContextMenu({
                          kind: 'media',
                          label: `Open · ${currentTrack.source}`,
                        });
                      }}
                      className="flex items-center"
                    >
                      <div className="relative flex h-[36px] items-center gap-2.5 rounded-[6px] bg-white/[0.09] pl-3.5 pr-3 transition-colors">
                        <span
                          className="absolute left-1 h-[14px] w-[3px] rounded-[1.5px]"
                          style={{ backgroundColor: accentColor }}
                        />
                        <div
                          className="h-[28px] w-[28px] shrink-0 rounded-[4px] border border-white/10"
                          style={{ background: currentTrack.artGradient }}
                        />
                        <div className="flex h-[14px] items-end gap-[2px]">
                          {[12, 7, 14, 9].map((h, idx) => (
                            <span
                              key={idx}
                              className="w-[3px] rounded-[1.5px] transition-all duration-200"
                              style={{
                                height: playing ? `${h}px` : '3.5px',
                                backgroundColor: accentColor,
                              }}
                            />
                          ))}
                        </div>
                        <div className="max-w-[175px] truncate text-[13px] font-semibold text-white">
                          {currentTrack.title}{' '}
                          <span
                            className="font-normal"
                            style={{ color: secondaryText }}
                          >
                            · {currentTrack.artist}
                          </span>
                        </div>
                      </div>

                      <div className="mx-1 h-[20px] w-[1px] bg-white/[0.08]" />

                      <div
                        className="flex items-center gap-0.5 px-1"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <button
                          onClick={() =>
                            setTrackIdx(
                              (i) => (i - 1 + TRACKS.length) % TRACKS.length
                            )
                          }
                          title="Previous"
                          aria-label="Previous"
                          className="flex h-[28px] w-[28px] items-center justify-center rounded-[4px] text-white transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98]"
                        >
                          <SkipBack className="h-3.5 w-3.5 fill-current" />
                        </button>
                        <button
                          onClick={() => setPlaying((p) => !p)}
                          title={playing ? 'Pause' : 'Play'}
                          aria-label={playing ? 'Pause' : 'Play'}
                          className="flex h-[28px] w-[28px] items-center justify-center rounded-[4px] bg-white/[0.06] text-white transition-all duration-150 hover:bg-white/[0.09] active:scale-[0.98]"
                        >
                          {playing ? (
                            <Pause className="h-3.5 w-3.5 fill-current" />
                          ) : (
                            <Play className="h-3.5 w-3.5 fill-current" />
                          )}
                        </button>
                        <button
                          onClick={() =>
                            setTrackIdx((i) => (i + 1) % TRACKS.length)
                          }
                          title="Next"
                          aria-label="Next"
                          className="flex h-[28px] w-[28px] items-center justify-center rounded-[4px] text-white transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98]"
                        >
                          <SkipForward className="h-3.5 w-3.5 fill-current" />
                        </button>
                      </div>
                    </div>
                  ) : (
                    <div
                      onContextMenu={(e) => {
                        e.preventDefault();
                        setContextMenu({ kind: 'shelf', label: 'Clear All' });
                      }}
                      className="relative flex h-[36px] cursor-pointer items-center gap-2.5 rounded-[6px] bg-white/[0.09] pl-3.5 pr-3 transition-colors"
                    >
                      <span
                        className="absolute left-1 h-[14px] w-[3px] rounded-[1.5px]"
                        style={{ backgroundColor: accentColor }}
                      />
                      <div className="flex h-[28px] w-[28px] items-center justify-center rounded-[4px] bg-white/[0.06]">
                        <FileText className="h-3.5 w-3.5 text-[#D6D6D6]" />
                      </div>
                      <span className="text-[13px] font-semibold text-white">
                        Shelf
                      </span>
                      <span
                        className="text-[12.5px] tabular-nums"
                        style={{ color: secondaryText }}
                      >
                        {shelfItems.length} files
                      </span>
                    </div>
                  )}
                </div>

                {/* State selector pills */}
                <div className="flex items-center gap-1.5 rounded-[6px] border border-white/10 bg-white/[0.03] p-1 text-[12px]">
                  <span className="px-2 text-white/50">Active Module:</span>
                  {(['media', 'clipboard', 'shelf'] as const).map((mode) => (
                    <button
                      key={mode}
                      onClick={() => setActiveBarRow(mode)}
                      className="rounded-[4px] px-2.5 py-1 capitalize transition-colors"
                      style={{
                        background:
                          activeBarRow === mode
                            ? 'rgba(255, 255, 255, 0.12)'
                            : 'transparent',
                        color: activeBarRow === mode ? '#fff' : secondaryText,
                      }}
                    >
                      {mode}
                    </button>
                  ))}
                </div>
              </div>
            </section>

            {/* Section 2: Floating Panels */}
            <section className="space-y-4">
              <div className="border-b border-white/[0.08] pb-1.5 text-[12px] font-semibold uppercase tracking-[0.06em] text-white/60">
                Floating Panels — Clipboard Stash, Shelf, Media Player &amp; Weather Forecast
              </div>

              <div className="grid grid-cols-1 items-start gap-7 md:grid-cols-2 xl:grid-cols-3">
                {/* 1. Clipboard Stash Panel */}
                <div
                  className="w-full max-w-[360px] overflow-hidden rounded-[8px] border p-4"
                  style={{
                    background: panelBg,
                    borderColor: outerBorder,
                    backdropFilter: backdropBlur,
                    boxShadow:
                      '0 12px 32px rgba(0, 0, 0, 0.28), 0 2px 6px rgba(0, 0, 0, 0.18)',
                  }}
                >
                  <div className="mb-3.5 flex items-center justify-between">
                    <div className="flex items-baseline gap-2">
                      <span className="text-[16px] font-semibold text-white">
                        Clipboard
                      </span>
                      <span
                        className="text-[12px] tabular-nums"
                        style={{ color: secondaryText }}
                      >
                        {clips.length} items
                      </span>
                    </div>
                    <button
                      onClick={() => setClips([])}
                      className="rounded-[4px] px-2.5 py-1 text-[12px] transition-all duration-150 hover:bg-white/[0.07] hover:text-white active:scale-[0.98] active:bg-white/[0.04]"
                      style={{ color: secondaryText }}
                    >
                      Clear All
                    </button>
                  </div>

                  {clips.length === 0 ? (
                    <div className="flex flex-col items-center justify-center py-10 text-center">
                      <p className="text-[13px]" style={{ color: secondaryText }}>
                        Clipboard is empty
                      </p>
                      <button
                        onClick={() => {
                          setClips(INITIAL_CLIPS);
                          setSelectedClipId(1);
                        }}
                        className="mt-3 rounded-[4px] bg-white/[0.06] px-3 py-1 text-[12px] text-white hover:bg-white/[0.09]"
                      >
                        Restore sample items
                      </button>
                    </div>
                  ) : (
                    <div className="grid grid-cols-3 gap-2">
                      {clips.map((clip) => {
                        const selected = clip.id === selectedClipId;
                        return (
                          <div
                            key={clip.id}
                            onClick={() => setSelectedClipId(clip.id)}
                            role="button"
                            tabIndex={0}
                            aria-label={`Clipboard item ${clip.resolution}`}
                            className="group relative aspect-[4/3] cursor-pointer overflow-hidden rounded-[4px] border transition-all duration-150 active:scale-[0.98]"
                            style={{
                              background: clip.gradient,
                              borderColor: 'rgba(255, 255, 255, 0.07)',
                              boxShadow: selected
                                ? `inset 0 0 0 1.5px ${accentColor}`
                                : 'none',
                            }}
                          >
                            {clip.label && (
                              <div className="p-2 text-[10px] leading-tight text-white/85">
                                {clip.label}
                              </div>
                            )}

                            {/* Pixel size in small dark label at bottom-left */}
                            <span className="absolute bottom-1.5 left-1.5 rounded-[4px] bg-black/65 px-1.5 py-0.5 text-[11px] tabular-nums text-white">
                              {clip.resolution}
                            </span>

                            {/* Selected check badge */}
                            {selected && (
                              <span
                                className="absolute right-1.5 top-1.5 flex h-[18px] w-[18px] items-center justify-center rounded-full text-black"
                                style={{ backgroundColor: accentColor }}
                              >
                                <Check className="h-3 w-3 stroke-[2.5]" />
                              </span>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>

                {/* 2. Shelf Panel */}
                <div
                  className="w-full max-w-[360px] overflow-hidden rounded-[8px] border"
                  style={{
                    background: panelBg,
                    borderColor: outerBorder,
                    backdropFilter: backdropBlur,
                    boxShadow:
                      '0 12px 32px rgba(0, 0, 0, 0.28), 0 2px 6px rgba(0, 0, 0, 0.18)',
                  }}
                >
                  <div className="p-4 pb-3">
                    <div className="mb-3.5 flex items-center justify-between">
                      <div className="flex items-baseline gap-2">
                        <span className="text-[16px] font-semibold text-white">
                          Shelf
                        </span>
                        <span
                          className="text-[12px] tabular-nums"
                          style={{ color: secondaryText }}
                        >
                          {shelfItems.length} files
                        </span>
                      </div>

                      <div className="flex items-center gap-1">
                        <button
                          onClick={() => setShelfPinned((p) => !p)}
                          title="Pin shelf card"
                          aria-label="Pin shelf card"
                          className="flex h-[28px] w-[28px] items-center justify-center rounded-[4px] transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98]"
                          style={{
                            background: shelfPinned
                              ? 'rgba(255, 255, 255, 0.09)'
                              : 'transparent',
                            color: shelfPinned ? accentColor : '#D6D6D6',
                          }}
                        >
                          <Pin className="h-3.5 w-3.5" />
                        </button>
                        <button
                          onClick={() => setShelfItems([])}
                          className="rounded-[4px] px-2.5 py-1 text-[12px] transition-all duration-150 hover:bg-white/[0.07] hover:text-white active:scale-[0.98]"
                          style={{ color: secondaryText }}
                        >
                          Clear All
                        </button>
                      </div>
                    </div>

                    {shelfItems.length === 0 ? (
                      <div className="flex flex-col items-center justify-center py-10 text-center">
                        <p className="text-[13px]" style={{ color: secondaryText }}>
                          Drop files here
                        </p>
                        <button
                          onClick={() => setShelfItems(INITIAL_SHELF)}
                          className="mt-3 rounded-[4px] bg-white/[0.06] px-3 py-1 text-[12px] text-white hover:bg-white/[0.09]"
                        >
                          Restore sample files
                        </button>
                      </div>
                    ) : (
                      <div className="grid grid-cols-3 gap-2">
                        {sortedShelf.map((item) => (
                          <div
                            key={item.id}
                            onClick={() =>
                              setShelfItems((prev) =>
                                prev.map((it) =>
                                  it.id === item.id
                                    ? { ...it, pinned: !it.pinned }
                                    : it
                                )
                              )
                            }
                            title={item.fullName}
                            role="button"
                            tabIndex={0}
                            aria-label={item.fullName}
                            className="relative flex min-h-[96px] cursor-pointer flex-col items-center justify-between rounded-[4px] border p-2.5 text-center transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98]"
                            style={{
                              background: item.pinned
                                ? 'rgba(255, 255, 255, 0.09)'
                                : 'rgba(255, 255, 255, 0.05)',
                              borderColor: 'rgba(255, 255, 255, 0.07)',
                            }}
                          >
                            {item.pinned && (
                              <Pin
                                className="absolute right-1.5 top-1.5 h-3 w-3"
                                style={{ color: accentColor }}
                              />
                            )}
                            <div className="my-auto flex h-8 items-center justify-center">
                              <FileText
                                className="h-7 w-7"
                                style={{
                                  color:
                                    item.kind === 'pdf' ? '#FF99A4' : '#D6D6D6',
                                }}
                              />
                            </div>
                            <div className="line-clamp-2 w-full break-all text-[12px] leading-[15px] text-white">
                              {item.name}
                            </div>
                          </div>
                        ))}
                      </div>
                    )}
                  </div>

                  {/* Shelf Footer Band (black 22%, 1px white 6% top border) */}
                  <div
                    className="flex items-center justify-between border-t px-4 py-2.5"
                    style={{
                      background: 'rgba(0, 0, 0, 0.22)',
                      borderColor: 'rgba(255, 255, 255, 0.06)',
                    }}
                  >
                    <div className="flex flex-wrap items-center gap-1.5">
                      <button
                        onClick={() => triggerShelfTool('ZIP ready')}
                        className="flex items-center gap-1.5 rounded-[4px] px-2.5 py-1.5 text-[12px] text-white transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98] active:bg-white/[0.04]"
                      >
                        <FolderArchive className="h-3.5 w-3.5 text-[#D6D6D6]" />
                        <span>{shelfToast === 'ZIP ready' ? 'ZIP ready' : 'Create ZIP'}</span>
                      </button>
                      <button
                        onClick={() => triggerShelfTool('Sharing…')}
                        className="flex items-center gap-1.5 rounded-[4px] px-2.5 py-1.5 text-[12px] text-white transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98] active:bg-white/[0.04]"
                      >
                        <Share2 className="h-3.5 w-3.5 text-[#D6D6D6]" />
                        <span>Share</span>
                      </button>
                      <button
                        onClick={() => triggerShelfTool('Compressed')}
                        className="flex items-center gap-1.5 rounded-[4px] px-2.5 py-1.5 text-[12px] text-white transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98] active:bg-white/[0.04]"
                      >
                        <Minimize2 className="h-3.5 w-3.5 text-[#D6D6D6]" />
                        <span>Compress</span>
                      </button>
                    </div>
                  </div>
                </div>

                {/* 3. Media Player Panel */}
                <div
                  className="w-full max-w-[360px] overflow-hidden rounded-[8px] border transition-colors duration-300"
                  style={{
                    background:
                      currentTrack.hasArt && !solidFallback
                        ? `linear-gradient(135deg, ${currentTrack.tintHex} 0%, rgba(32, 32, 32, 0.85) 65%), ${panelBg}`
                        : panelBg,
                    borderColor: outerBorder,
                    backdropFilter: backdropBlur,
                    boxShadow:
                      '0 12px 32px rgba(0, 0, 0, 0.28), 0 2px 6px rgba(0, 0, 0, 0.18)',
                  }}
                >
                  {showOutputPicker ? (
                    <div className="p-4">
                      <div className="mb-4 flex items-center justify-between">
                        <div className="flex items-center gap-2.5">
                          <button
                            onClick={() => setShowOutputPicker(false)}
                            title="Back"
                            aria-label="Back"
                            className="flex h-8 w-8 items-center justify-center rounded-[4px] bg-white/[0.06] text-white hover:bg-white/[0.09] active:scale-[0.98]"
                          >
                            ←
                          </button>
                          <div>
                            <div className="text-[15px] font-semibold text-white">
                              Audio Output
                            </div>
                            <div
                              className="text-[12px]"
                              style={{ color: secondaryText }}
                            >
                              {currentTrack.title} · {currentTrack.artist}
                            </div>
                          </div>
                        </div>
                        <button
                          title="Audio Settings"
                          aria-label="Audio Settings"
                          className="flex h-8 w-8 items-center justify-center rounded-[4px] bg-white/[0.05] text-[#D6D6D6] hover:bg-white/[0.09]"
                        >
                          <Settings className="h-4 w-4" />
                        </button>
                      </div>

                      {/* Active output row */}
                      <div className="mb-3 rounded-[4px] border border-white/[0.07] bg-white/[0.09] p-3">
                        <div className="mb-2.5 flex items-center justify-between">
                          <div className="flex items-center gap-2.5">
                            <div
                              className="flex h-9 w-9 items-center justify-center rounded-[4px] text-black"
                              style={{ backgroundColor: accentColor }}
                            >
                              <Headphones className="h-4 w-4" />
                            </div>
                            <div>
                              <div className="text-[14px] font-semibold text-white">
                                {selectedOutput}
                              </div>
                              <div
                                className="text-[11.5px]"
                                style={{ color: secondaryText }}
                              >
                                Bluetooth · Left 85% · Right 80%
                              </div>
                            </div>
                          </div>
                          <span
                            className="flex h-5 w-5 items-center justify-center rounded-full text-black"
                            style={{ backgroundColor: accentColor }}
                          >
                            <Check className="h-3 w-3 stroke-[2.5]" />
                          </span>
                        </div>

                        <input
                          type="range"
                          min={0}
                          max={100}
                          value={volumePct}
                          onChange={(e) => setVolumePct(Number(e.target.value))}
                          className="w-full accent-[#60CDFF]"
                        />
                      </div>

                      {/* Other devices */}
                      {[
                        {
                          name: 'Speakers (Realtek Audio)',
                          sub: 'Built-in',
                        },
                        {
                          name: 'LG UltraGear (NVIDIA High Definition)',
                          sub: 'Display Audio',
                        },
                      ].map((dev) => (
                        <button
                          key={dev.name}
                          onClick={() => {
                            setSelectedOutput(dev.name);
                            setShowOutputPicker(false);
                          }}
                          className="mb-1.5 flex w-full items-center gap-2.5 rounded-[4px] border border-white/[0.07] bg-white/[0.05] p-2.5 text-left transition-colors hover:bg-white/[0.07]"
                        >
                          <div className="flex h-8 w-8 items-center justify-center rounded-[4px] bg-white/[0.08] text-[#D6D6D6]">
                            <Volume2 className="h-4 w-4" />
                          </div>
                          <div className="min-w-0 flex-1">
                            <div className="truncate text-[13px] font-medium text-white">
                              {dev.name}
                            </div>
                            <div
                              className="text-[11.5px]"
                              style={{ color: secondaryText }}
                            >
                              {dev.sub}
                            </div>
                          </div>
                        </button>
                      ))}
                    </div>
                  ) : (
                    <>
                      <div className="p-4">
                        {/* Top: 96px Album Art + Title/Artist/Equalizer + Pin */}
                        <div className="relative mb-4 flex gap-3.5">
                          <div
                            className="flex h-[96px] w-[96px] shrink-0 items-center justify-center rounded-[4px] border border-white/[0.08]"
                            style={{
                              background: currentTrack.artGradient,
                              boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
                            }}
                          >
                            {!currentTrack.hasArt && (
                              <span className="text-[28px] font-semibold text-white/40">
                                S
                              </span>
                            )}
                          </div>

                          <div className="flex min-w-0 flex-1 flex-col justify-center pr-8">
                            <div className="truncate text-[20px] font-semibold leading-[26px] tracking-[-0.01em] text-white">
                              {currentTrack.title}
                            </div>
                            <div
                              className="mt-0.5 truncate text-[13px]"
                              style={{ color: secondaryText }}
                            >
                              {currentTrack.artist}
                            </div>

                            {/* 4-bar Equalizer below artist */}
                            <div className="mt-2.5 flex h-[14px] items-end gap-[2px]">
                              {[13, 7, 11, 5].map((h, idx) => (
                                <span
                                  key={idx}
                                  className="w-[3px] rounded-[1.5px] transition-all duration-200"
                                  style={{
                                    height: playing ? `${h}px` : '3.5px',
                                    backgroundColor: accentColor,
                                  }}
                                />
                              ))}
                            </div>
                          </div>

                          <button
                            onClick={() => setMediaPinned((p) => !p)}
                            title="Pin card"
                            aria-label="Pin card"
                            className="absolute right-0 top-0 flex h-[32px] w-[32px] items-center justify-center rounded-[4px] transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98]"
                            style={{
                              background: mediaPinned
                                ? 'rgba(255, 255, 255, 0.09)'
                                : 'transparent',
                              color: mediaPinned ? accentColor : '#D6D6D6',
                            }}
                          >
                            <Pin className="h-3.5 w-3.5" />
                          </button>
                        </div>

                        {/* Quiet Source & Output Chips */}
                        <div className="mb-4 flex items-center justify-between gap-2">
                          <div
                            className="flex items-center gap-1.5 rounded-[4px] border border-white/[0.05] bg-white/[0.04] px-2.5 py-1 text-[12px]"
                            style={{ color: secondaryText }}
                          >
                            <span
                              className="h-2 w-2 rounded-full"
                              style={{ backgroundColor: currentTrack.sourceColor }}
                            />
                            <span>{currentTrack.source}</span>
                          </div>

                          <button
                            onClick={() => setShowOutputPicker(true)}
                            title="Change audio output"
                            className="flex items-center gap-1.5 rounded-[4px] border border-white/[0.05] bg-white/[0.04] px-2.5 py-1 text-[12px] transition-colors hover:bg-white/[0.07]"
                            style={{ color: secondaryText }}
                          >
                            <Headphones className="h-3.5 w-3.5 text-[#D6D6D6]" />
                            <span className="max-w-[150px] truncate">
                              {selectedOutput}
                            </span>
                          </button>
                        </div>

                        {/* Slider: 4px track + 20px thumb with #454545 ring and accent center */}
                        <div>
                          <div
                            className="group relative flex h-5 cursor-pointer items-center"
                            onClick={(e) => {
                              const rect = e.currentTarget.getBoundingClientRect();
                              const frac = Math.max(
                                0,
                                Math.min(1, (e.clientX - rect.left) / rect.width)
                              );
                              setPositionSec(
                                Math.round(frac * currentTrack.durationSec)
                              );
                            }}
                          >
                            <div className="h-[4px] w-full overflow-hidden rounded-[2px] bg-white/20">
                              <div
                                className="h-full rounded-[2px]"
                                style={{
                                  width: `${
                                    (positionSec / currentTrack.durationSec) * 100
                                  }%`,
                                  backgroundColor: accentColor,
                                }}
                              />
                            </div>
                            <div
                              className="absolute top-1/2 flex h-[20px] w-[20px] -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border border-white/[0.18] bg-[#454545] shadow transition-transform group-hover:scale-105"
                              style={{
                                left: `${
                                  (positionSec / currentTrack.durationSec) * 100
                                }%`,
                              }}
                            >
                              <span
                                className="h-[10px] w-[10px] rounded-full transition-all group-hover:h-[12px] group-hover:w-[12px]"
                                style={{ backgroundColor: accentColor }}
                              />
                            </div>
                          </div>

                          <div
                            className="mt-1 flex justify-between text-[12px] tabular-nums"
                            style={{ color: secondaryText }}
                          >
                            <span>{formatTime(positionSec)}</span>
                            <span>{formatTime(currentTrack.durationSec)}</span>
                          </div>
                        </div>
                      </div>

                      {/* Footer Band Transport Controls */}
                      <div
                        className="flex items-center justify-between border-t px-6 py-3"
                        style={{
                          background: 'rgba(0, 0, 0, 0.22)',
                          borderColor: 'rgba(255, 255, 255, 0.06)',
                        }}
                      >
                        <button
                          onClick={() => setShuffleOn((s) => !s)}
                          title="Shuffle"
                          aria-label="Shuffle"
                          className="flex h-[40px] w-[40px] items-center justify-center rounded-[4px] transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98]"
                          style={{
                            color: shuffleOn ? accentColor : secondaryText,
                            background: shuffleOn
                              ? 'rgba(255,255,255,0.06)'
                              : 'transparent',
                          }}
                        >
                          <Shuffle className="h-4 w-4" />
                        </button>

                        <button
                          onClick={() => {
                            const next =
                              (trackIdx - 1 + TRACKS.length) % TRACKS.length;
                            setTrackIdx(next);
                            setPositionSec(TRACKS[next].positionSec);
                          }}
                          title="Previous"
                          aria-label="Previous"
                          className="flex h-[40px] w-[40px] items-center justify-center rounded-[4px] text-white transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98]"
                        >
                          <SkipBack className="h-4 w-4 fill-current" />
                        </button>

                        {/* 48px Accent Circle Play/Pause with Soft Accent Glow */}
                        <button
                          onClick={() => setPlaying((p) => !p)}
                          title={playing ? 'Pause' : 'Play'}
                          aria-label={playing ? 'Pause' : 'Play'}
                          className="flex h-[48px] w-[48px] items-center justify-center rounded-full text-black transition-all duration-150 hover:brightness-110 active:scale-[0.96]"
                          style={{
                            backgroundColor: accentColor,
                            boxShadow: playing
                              ? `0 0 18px ${accentColor}80`
                              : 'none',
                          }}
                        >
                          {playing ? (
                            <Pause className="h-5 w-5 fill-current" />
                          ) : (
                            <Play className="h-5 w-5 fill-current" />
                          )}
                        </button>

                        <button
                          onClick={() => {
                            const next = (trackIdx + 1) % TRACKS.length;
                            setTrackIdx(next);
                            setPositionSec(TRACKS[next].positionSec);
                          }}
                          title="Next"
                          aria-label="Next"
                          className="flex h-[40px] w-[40px] items-center justify-center rounded-[4px] text-white transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98]"
                        >
                          <SkipForward className="h-4 w-4 fill-current" />
                        </button>

                        <button
                          onClick={() =>
                            setRepeatMode((m) => ((m + 1) % 3) as 0 | 1 | 2)
                          }
                          title={
                            repeatMode === 2
                              ? 'Repeat One (1)'
                              : repeatMode === 1
                              ? 'Repeat All'
                              : 'Repeat Off'
                          }
                          aria-label="Repeat"
                          className="relative flex h-[40px] w-[40px] items-center justify-center rounded-[4px] transition-all duration-150 hover:bg-white/[0.07] active:scale-[0.98]"
                          style={{
                            color: repeatMode > 0 ? accentColor : secondaryText,
                            background:
                              repeatMode > 0
                                ? 'rgba(255,255,255,0.06)'
                                : 'transparent',
                          }}
                        >
                          <Repeat className="h-4 w-4" />
                          {repeatMode === 2 && (
                            <span
                              className="absolute text-[9.5px] font-bold leading-none"
                              style={{ color: accentColor }}
                            >
                              1
                            </span>
                          )}
                        </button>
                      </div>
                    </>
                  )}
                </div>

                {/* 4. Weather Panel (Overcast / Rain Sky Tint) */}
                <div
                  className="w-full max-w-[360px] overflow-hidden rounded-[8px] border p-4 transition-colors duration-300"
                  style={{
                    background: !solidFallback
                      ? weatherMode === 'overcast'
                        ? `linear-gradient(180deg, rgba(80, 95, 115, 0.24) 0%, rgba(32, 32, 32, 0.85) 100%), ${panelBg}`
                        : `linear-gradient(180deg, rgba(45, 75, 130, 0.24) 0%, rgba(32, 32, 32, 0.85) 100%), ${panelBg}`
                      : panelBg,
                    borderColor: outerBorder,
                    backdropFilter: backdropBlur,
                    boxShadow:
                      '0 12px 32px rgba(0, 0, 0, 0.28), 0 2px 6px rgba(0, 0, 0, 0.18)',
                  }}
                >
                  <div className="mb-3 flex items-center justify-between">
                    <span className="text-[11px] uppercase tracking-wider text-white/50">
                      Sky Tint (24%)
                    </span>
                    <div className="flex gap-1">
                      <button
                        onClick={() => setWeatherMode('overcast')}
                        className="rounded-[4px] px-2 py-0.5 text-[11px] transition-colors"
                        style={{
                          background:
                            weatherMode === 'overcast'
                              ? 'rgba(255,255,255,0.09)'
                              : 'transparent',
                          color:
                            weatherMode === 'overcast' ? '#fff' : secondaryText,
                        }}
                      >
                        Overcast (15°)
                      </button>
                      <button
                        onClick={() => setWeatherMode('rain')}
                        className="rounded-[4px] px-2 py-0.5 text-[11px] transition-colors"
                        style={{
                          background:
                            weatherMode === 'rain'
                              ? 'rgba(255,255,255,0.09)'
                              : 'transparent',
                          color: weatherMode === 'rain' ? '#fff' : secondaryText,
                        }}
                      >
                        Rain (12°)
                      </button>
                    </div>
                  </div>

                  {/* Weather Header */}
                  <div className="mb-5 flex items-start justify-between">
                    <div>
                      <div className="text-[16px] font-semibold text-white">
                        Giridih
                      </div>
                      <div
                        className="mt-1 text-[13px]"
                        style={{ color: secondaryText }}
                      >
                        {weatherMode === 'overcast'
                          ? 'Overcast · 12° / 15°'
                          : 'Heavy Rain · 10° / 14°'}
                      </div>
                    </div>

                    <div className="text-right">
                      <div className="flex items-center justify-end gap-2">
                        {weatherMode === 'overcast' ? (
                          <Cloud className="h-8 w-8 text-[#D6D6D6]" />
                        ) : (
                          <CloudRain className="h-8 w-8 text-[#D6D6D6]" />
                        )}
                        <span
                          className="text-[48px] font-light leading-none tracking-[-0.03em] tabular-nums text-white"
                          style={{
                            fontFamily:
                              '"Segoe UI Variable Display", "Segoe UI", sans-serif',
                          }}
                        >
                          {weatherMode === 'overcast' ? '15°' : '12°'}
                        </span>
                      </div>
                      <div
                        className="mt-1 text-[12px]"
                        style={{ color: secondaryText }}
                      >
                        Now
                      </div>
                    </div>
                  </div>

                  {/* Hourly Forecast: Plain Row Separated by 1px Thin Divider (No Box) */}
                  <div className="grid grid-cols-6 gap-1 border-t border-white/[0.07] pt-3.5">
                    {(weatherMode === 'overcast'
                      ? [
                          { h: '18:00', icon: 'cloud', t: '15°' },
                          { h: '19:00', icon: 'cloud', t: '15°' },
                          { h: '20:00', icon: 'cloud', t: '14°' },
                          { h: '21:00', icon: 'rain', t: '14°' },
                          { h: '22:00', icon: 'cloud', t: '14°' },
                          { h: '23:00', icon: 'cloud', t: '14°' },
                        ]
                      : [
                          { h: '14:00', icon: 'rain', t: '12°' },
                          { h: '15:00', icon: 'rain', t: '12°' },
                          { h: '16:00', icon: 'rain', t: '11°' },
                          { h: '17:00', icon: 'cloud', t: '11°' },
                          { h: '18:00', icon: 'cloud', t: '10°' },
                          { h: '19:00', icon: 'sun', t: '10°' },
                        ]
                    ).map((slot, idx) => {
                      const active = idx === selectedHourIdx;
                      return (
                        <button
                          key={slot.h}
                          onClick={() => setSelectedHourIdx(idx)}
                          className="relative flex flex-col items-center gap-2 rounded-[4px] px-1 py-2 transition-colors hover:bg-white/[0.07]"
                          style={{
                            background: active
                              ? 'rgba(255, 255, 255, 0.09)'
                              : 'transparent',
                          }}
                        >
                          <span
                            className="text-[12px] tabular-nums"
                            style={{ color: secondaryText }}
                          >
                            {slot.h}
                          </span>
                          {slot.icon === 'rain' ? (
                            <CloudRain className="h-5 w-5 text-[#D6D6D6]" />
                          ) : slot.icon === 'sun' ? (
                            <Sun className="h-5 w-5 text-[#D6D6D6]" />
                          ) : (
                            <Cloud className="h-5 w-5 text-[#D6D6D6]" />
                          )}
                          <span className="text-[13px] font-semibold tabular-nums text-white">
                            {slot.t}
                          </span>
                          {active && (
                            <span
                              className="absolute bottom-0.5 h-[3px] w-[16px] rounded-[2px]"
                              style={{ backgroundColor: accentColor }}
                            />
                          )}
                        </button>
                      );
                    })}
                  </div>
                </div>

                {/* 5. Redesigned Windows 11 Settings UI (nowbar-settings) */}
                <div
                  className="w-full max-w-[360px] overflow-hidden rounded-[8px] border p-4"
                  style={{
                    background: panelBg,
                    borderColor: outerBorder,
                    backdropFilter: backdropBlur,
                    boxShadow:
                      '0 12px 32px rgba(0, 0, 0, 0.28), 0 2px 6px rgba(0, 0, 0, 0.18)',
                  }}
                >
                  <div className="mb-3 flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <Settings className="h-4 w-4" style={{ color: accentColor }} />
                      <span className="text-[15px] font-semibold text-white">
                        Syro Settings (Win11 UI)
                      </span>
                    </div>
                    <span className="rounded-[4px] bg-white/[0.06] px-2 py-0.5 text-[11px] text-white/70">
                      Streamlined
                    </span>
                  </div>

                  {/* Live Taskbar Mockup inside Settings */}
                  <div className="mb-3.5 rounded-[8px] border border-white/10 bg-[#181818] p-2.5">
                    <div className="mb-2 text-[11px] text-white/60">
                      Live Taskbar Preview · Corner Weather+Earbuds + Pill
                    </div>
                    <div className="flex items-center gap-1.5">
                      <div className="flex h-[32px] items-center gap-1.5 rounded-[6px] border border-white/10 bg-[#202020] px-2 text-[11px] font-semibold">
                        <Cloud className="h-3 w-3" style={{ color: accentColor }} />
                        <span>24°</span>
                        <span className="mx-0.5 h-3 w-[1px] bg-white/15" />
                        <Headphones className="h-3 w-3 text-[#D6D6D6]" />
                        <span>100%</span>
                      </div>
                      <div className="relative flex h-[32px] flex-1 items-center gap-2 rounded-[6px] border border-white/10 bg-[#202020] pl-3 pr-2 text-[11px]">
                        <span
                          className="absolute left-1 h-3 w-[2.5px] rounded-full"
                          style={{ backgroundColor: accentColor }}
                        />
                        <span className="truncate font-semibold text-white">
                          {currentTrack.title}
                        </span>
                      </div>
                    </div>
                  </div>

                  {/* Updated Pill Sizes: Compact (78%), Normal (88%), Large (100% = old Normal) */}
                  <div className="mb-2 text-[12px] font-medium text-white/80">
                    Pill Size (Old Normal is now Large)
                  </div>
                  <div className="grid grid-cols-3 gap-2">
                    {[
                      { label: 'Compact', sub: '234×34 · 78%' },
                      { label: 'Normal', sub: '264×38 · 88%' },
                      { label: 'Large', sub: '300×44 · 100%' },
                    ].map((sz, i) => (
                      <button
                        key={sz.label}
                        onClick={() => setPillSizeIdx(i as 0 | 1 | 2)}
                        className="rounded-[6px] border p-2 text-left transition-colors"
                        style={{
                          borderColor:
                            pillSizeIdx === i
                              ? accentColor
                              : 'rgba(255,255,255,0.08)',
                          background:
                            pillSizeIdx === i
                              ? 'rgba(255,255,255,0.08)'
                              : 'rgba(255,255,255,0.03)',
                        }}
                      >
                        <div className="text-[12px] font-semibold text-white">
                          {sz.label}
                        </div>
                        <div className="text-[10.5px] text-white/60">{sz.sub}</div>
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            </section>
          </div>
        )}
      </main>
    </div>
  );
}
