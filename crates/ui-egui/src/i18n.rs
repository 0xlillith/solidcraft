//! Multi-language (i18n) static translation dictionary for SolveCraft.
//!
//! Provides zero-overhead, completely offline local string translations.

/// Supported languages: (id, display_name).
pub const LANGUAGES: [(&str, &str); 2] = [("en", "English"), ("ja", "日本語")];

/// English to Japanese dictionary.
const JA_TRANSLATIONS: &[(&str, &str)] = &[
    // --- Common & Window controls ---
    ("Preferences", "環境設定"),
    ("General", "一般"),
    ("Navigation", "ナビゲーション"),
    ("Keyboard Shortcuts", "キーボードショートカット"),
    ("Design", "デザイン"),
    ("Graphics", "グラフィックス"),
    ("Sketch", "スケッチ"),
    ("Selection", "選択"),
    ("Language", "言語"),
    ("Theme", "テーマ"),
    ("UI scale", "UIスケール"),
    ("Units of new designs", "新規デザインの単位"),
    ("Autosave every", "自動保存の間隔"),
    ("Length decimals", "長さの小数点桁数"),
    ("Angle decimals", "角度の小数点桁数"),
    ("Agent cursor", "AIエージェントカーソル"),
    ("Agent cursor speed", "AIエージェント速度"),
    ("Restore Defaults", "初期値に戻す"),
    ("Reset the settings on this page", "このページの設定をリセット"),
    ("OK", "OK"),
    ("Cancel", "キャンセル"),
    ("Apply", "適用"),
    ("Save", "保存"),
    ("Don't Save", "保存しない"),
    ("Untitled", "無題"),
    ("Dark", "ダーク"),
    ("Light", "ライト"),
    ("System", "システム"),
    ("Help", "ヘルプ"),
    ("About", "SolveCraft について"),
    ("Close", "閉じる"),
    ("Delete", "削除"),
    ("Expand", "展開"),
    ("Collapse", "折りたたむ"),
    // --- Preferences Navigation & Design ---
    ("Pan, zoom, orbit", "パン、ズーム、オービット"),
    ("Reverse zoom direction", "ズーム方向を反転"),
    ("Zoom to the cursor", "カーソル位置へズーム"),
    ("Orbit around", "回転の中心"),
    ("View centre", "ビュー中心"),
    ("Point under the cursor", "カーソル下の点"),
    ("Animate view changes", "視点変更をアニメーション表示"),
    ("Animation speed", "アニメーション速度"),
    ("Default modelling orientation", "デフォルトのモデル方向"),
    ("Offer to stand up Y-up files", "Y-upファイルを立ち上げるか確認"),
    ("Visual style", "表示スタイル"),
    // --- Application bar & Menus ---
    ("File", "ファイル"),
    ("Edit", "編集"),
    ("Create", "作成"),
    ("Modify", "修正"),
    ("Assemble", "アセンブリ"),
    ("Construct", "構築"),
    ("Inspect", "検査"),
    ("Insert", "挿入"),
    ("Select", "選択"),
    ("New Design", "新規デザイン"),
    ("Open...", "開く..."),
    ("Open", "開く"),
    ("Save", "保存"),
    ("Save As...", "名前を付けて保存..."),
    ("Export...", "エクスポート..."),
    ("Undo", "元に戻す"),
    ("Redo", "やり直す"),
    ("Cut", "切り取り"),
    ("Copy", "コピー"),
    ("Paste", "貼り付け"),
    ("Clear Recent", "最近使った項目をクリア"),
    ("Open Recent", "最近使った項目を開く"),
    ("Quit", "終了"),
    // --- Toolbar tabs ---
    ("SOLID", "ソリッド"),
    ("SURFACE", "サーフェス"),
    ("MESH", "メッシュ"),
    ("SHEET METAL", "板金"),
    ("PLASTIC", "プラスチック"),
    ("SKETCH", "スケッチ"),
    ("CREATE", "作成"),
    ("MODIFY", "修正"),
    ("ASSEMBLE", "アセンブリ"),
    ("CONSTRUCT", "構築"),
    ("INSPECT", "検査"),
    ("INSERT", "挿入"),
    ("SELECT", "選択"),
    ("FINISH SKETCH", "スケッチを終了"),
    ("Finish Sketch", "スケッチを終了"),
    // --- Sketch Palette ---
    ("SKETCH PALETTE", "スケッチ パレット"),
    ("Options", "オプション"),
    ("Linetype", "線種"),
    ("Normal", "標準"),
    ("Construction", "構築"),
    ("Centerline", "中心線"),
    ("Look At", "視点を合わせる"),
    ("Sketch Grid", "スケッチ グリッド"),
    ("Snap", "スナップ"),
    ("Slice", "スライス"),
    ("Show Profile", "プロファイルを表示"),
    ("Show Points", "点を表示"),
    ("Show Dimensions", "寸法を表示"),
    ("Show Constraints", "拘束を表示"),
    ("Show Projected Geometries", "投影ジオメトリを表示"),
    ("3D Sketch", "3D スケッチ"),
    ("Fully constrained", "完全拘束"),
    ("1 degree of freedom", "自由度 1"),
    ("degrees of freedom", "自由度"),
    ("Constraints conflict", "拘束が競合しています"),
    // --- Dialog Titles (All Caps & Title Case) ---
    ("CREATE SKETCH", "スケッチを作成"),
    ("Create Sketch", "スケッチを作成"),
    ("EDIT", "編集"),
    ("EXTRUDE", "押し出し"),
    ("Extrude", "押し出し"),
    ("REVOLVE", "回転"),
    ("Revolve", "回転"),
    ("SWEEP", "スイープ"),
    ("Sweep", "スイープ"),
    ("LOFT", "ロフト"),
    ("Loft", "ロフト"),
    ("FILLET", "フィレット"),
    ("Fillet", "フィレット"),
    ("CHAMFER", "面取り"),
    ("Chamfer", "面取り"),
    ("SHELL", "シェル"),
    ("Shell", "シェル"),
    ("DRAFT", "勾配"),
    ("Draft", "勾配"),
    ("SCALE", "尺度"),
    ("Scale", "尺度"),
    ("COMBINE", "結合"),
    ("Combine", "結合"),
    ("SPLIT BODY", "ボディを分割"),
    ("Split Body", "ボディを分割"),
    ("MIRROR", "ミラー"),
    ("Mirror", "ミラー"),
    ("HOLE", "穴"),
    ("Hole", "穴"),
    ("THREAD", "ねじ"),
    ("Thread", "ねじ"),
    ("BOX", "直方体"),
    ("Box", "直方体"),
    ("CYLINDER", "円柱"),
    ("Cylinder", "円柱"),
    ("SPHERE", "球"),
    ("Sphere", "球"),
    ("TORUS", "トーラス"),
    ("Torus", "トーラス"),
    ("COIL", "コイル"),
    ("Coil", "コイル"),
    ("PIPE", "パイプ"),
    ("Pipe", "パイプ"),
    ("OFFSET PLANE", "オフセット平面"),
    ("Offset Plane", "オフセット平面"),
    ("PLANE AT ANGLE", "傾斜平面"),
    ("Plane at Angle", "傾斜平面"),
    ("MEASURE", "計測"),
    ("Measure", "計測"),
    ("SECTION ANALYSIS", "断面解析"),
    ("Section Analysis", "断面解析"),
    ("PARAMETERS", "パラメータ"),
    ("Parameters", "パラメータ"),
    ("MOVE", "移動"),
    ("Move", "移動"),
    ("RECTANGULAR PATTERN", "矩形パターン"),
    ("Rectangular Pattern", "矩形パターン"),
    ("CIRCULAR PATTERN", "円形パターン"),
    ("Circular Pattern", "円形パターン"),
    ("PATTERN ON PATH", "パス上のパターン"),
    ("Pattern on Path", "パス上のパターン"),
    // --- Dialog Row Labels and Inputs ---
    ("Plane", "平面"),
    ("Profile", "プロファイル"),
    ("Profiles", "プロファイル"),
    ("Axis", "軸"),
    ("Axes", "軸"),
    ("Path", "パス"),
    ("Points", "点"),
    ("Objects", "オブジェクト"),
    ("Bodies", "ボディ"),
    ("Features", "フィーチャー"),
    ("Start", "開始"),
    ("Offset", "オフセット"),
    ("Profile Plane", "プロファイル平面"),
    ("Direction", "方向"),
    ("One Side", "片側"),
    ("Two Sides", "両側"),
    ("Symmetric", "対称"),
    ("Extent Type", "範囲タイプ"),
    ("Distance", "距離"),
    ("Distance 1", "距離 1"),
    ("Distance 2", "距離 2"),
    ("To Object", "オブジェクトまで"),
    ("All", "すべて"),
    ("Taper Angle", "テーパ角度"),
    ("Operation", "操作"),
    ("New Body", "新規ボディ"),
    ("Join", "結合"),
    ("Cut", "切り取り"),
    ("Intersect", "交差"),
    ("Angle", "角度"),
    ("Radius", "半径"),
    ("Radius Type", "半径タイプ"),
    ("Constant", "一定"),
    ("Chord Length", "弦長"),
    ("Variable", "可変"),
    ("Start Radius", "開始半径"),
    ("End Radius", "終了半径"),
    ("Flip", "反転"),
    ("Thickness", "厚さ"),
    ("Placement", "配置"),
    ("Single", "単一"),
    ("Multiple", "複数"),
    ("Hole Type", "穴タイプ"),
    ("Simple", "単純"),
    ("Counterbore", "座ぐり"),
    ("Countersink", "皿もみ"),
    ("Hole Tap Type", "タップタイプ"),
    ("Tapped", "タップ立て"),
    ("Drill Point", "先端"),
    ("Flat", "フラット"),
    ("Tip Angle", "先端角度"),
    ("Diameter", "直径"),
    ("Depth", "深さ"),
    ("Object Type", "オブジェクトタイプ"),
    ("Clear the selection", "選択をクリア"),
    // --- Browser Tree ---
    ("Document Settings", "ドキュメントの設定"),
    ("Units", "単位"),
    ("Named Views", "名前付きビュー"),
    ("Origin", "原点"),
    ("Top", "上面"),
    ("Front", "正面"),
    ("Right", "右側面"),
    ("Home", "ホーム"),
    // --- Sketch commands & tools ---
    ("Line", "線分"),
    ("Rectangle", "長方形"),
    ("Circle", "円"),
    ("Arc", "円弧"),
    ("Polygon", "ポリゴン"),
    ("Ellipse", "楕円"),
    ("Slot", "スロット"),
    ("Spline", "スプライン"),
    ("Conic Curve", "円錐曲線"),
    ("Point", "点"),
    ("Text", "テキスト"),
    ("Trim", "トリム"),
    ("Extend", "延長"),
    ("Break", "分割"),
    ("Sketch Scale", "スケッチ尺度"),
    ("Project", "投影"),
    ("Dimension", "寸法"),
    ("Sketch Dimension", "スケッチ寸法"),
    ("Horizontal/Vertical", "水平/垂直"),
    ("Coincident", "一致"),
    ("Tangent", "正接"),
    ("Equal", "等しい"),
    ("Parallel", "平行"),
    ("Perpendicular", "直交"),
    ("Fix/Unfix", "固定/固定解除"),
    ("Midpoint", "中点"),
    ("Concentric", "同心円"),
    ("Collinear", "同一直線"),
    ("Symmetry", "対称"),
    ("Curvature", "曲率"),
    ("Smooth", "スムーズ"),
    // --- Workspace selector & environments ---
    ("DESIGN", "デザイン"),
    ("workspace", "ワークスペース"),
    ("Change workspace", "ワークスペースの変更"),
    ("Generative Design", "ジェネレーティブ デザイン"),
    ("Render", "レンダリング"),
    ("Animation", "アニメーション"),
    ("Simulation", "シミュレーション"),
    ("Manufacture", "製造"),
    ("Drawing", "図面"),
    ("coming later", "準備中"),
    ("current", "現在"),
    // --- Browser Tree ---
    ("Document Settings", "ドキュメントの設定"),
    ("Units", "単位"),
    ("Named Views", "名前付きビュー"),
    ("Origin", "原点"),
    ("Top", "上面"),
    ("Front", "正面"),
    ("Right", "右側面"),
    ("Home", "ホーム"),
    ("TOP", "上面"),
    ("FRONT", "正面"),
    ("RIGHT", "右側面"),
    ("HOME", "ホーム"),
    ("Bodies", "ボディ"),
    ("Sketches", "スケッチ"),
    ("Canvases", "キャンバス"),
    ("Joints", "ジョイント"),
    ("Construction", "構築"),
    ("Collapse", "折りたたむ"),
    ("Expand", "展開"),
    ("Hide", "非表示"),
    ("Show", "表示"),
    ("Active component", "アクティブなコンポーネント"),
    ("Activate", "アクティブ化"),
];

thread_local! {
    static CURRENT_LANG: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Sets the current UI language for the thread.
pub fn set_current_lang(lang: &str) {
    CURRENT_LANG.with(|l| {
        *l.borrow_mut() = lang.to_string();
    });
}

/// Gets the current UI language for the thread.
pub fn current_lang() -> String {
    CURRENT_LANG.with(|l| l.borrow().clone())
}

/// Translate text using the thread's current language.
pub fn tr(text: &str) -> &str {
    CURRENT_LANG.with(|l| {
        let lang = l.borrow();
        if lang.as_str() == "ja" { tr_for_lang("ja", text) } else { text }
    })
}

/// Translate text using a specified language.
pub fn tr_for_lang<'a>(lang: &str, text: &'a str) -> &'a str {
    if lang == "ja" {
        for &(en, ja) in JA_TRANSLATIONS {
            if en == text {
                return ja;
            }
        }
    }
    text
}

/// Dynamically translate labels including patterned names like "Sketch1", "Body2", "Units: mm", etc.
pub fn tr_dyn(text: &str) -> String {
    CURRENT_LANG.with(|l| {
        let lang = l.borrow();
        if lang.as_str() != "ja" {
            return text.to_string();
        }
        for &(en, ja) in JA_TRANSLATIONS {
            if en == text {
                return ja.to_string();
            }
        }
        if let Some(rest) = text.strip_prefix("Sketch")
            && !rest.is_empty()
            && rest.chars().all(|c| c.is_ascii_digit())
        {
            return format!("スケッチ{rest}");
        }
        if let Some(rest) = text.strip_prefix("Body")
            && !rest.is_empty()
            && rest.chars().all(|c| c.is_ascii_digit())
        {
            return format!("ボディ{rest}");
        }
        if let Some(rest) = text.strip_prefix("Component")
            && !rest.is_empty()
            && rest.chars().all(|c| c.is_ascii_digit())
        {
            return format!("コンポーネント{rest}");
        }
        if let Some(rest) = text.strip_prefix("Units: ") {
            return format!("単位: {rest}");
        }
        if let Some(rest) = text.strip_prefix("Precision: ") {
            return format!("精度: {rest}");
        }
        if let Some(rest) = text.strip_prefix("Parameters (") {
            return format!("パラメータ ({rest}");
        }
        text.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_japanese() {
        set_current_lang("ja");
        assert_eq!(tr("Preferences"), "環境設定");
        assert_eq!(tr("Extrude"), "押し出し");
        assert_eq!(tr("CREATE SKETCH"), "スケッチを作成");
        assert_eq!(tr("SKETCH PALETTE"), "スケッチ パレット");
        assert_eq!(tr("Finish Sketch"), "スケッチを終了");
        assert_eq!(tr("FINISH SKETCH"), "スケッチを終了");
        assert_eq!(tr("Generative Design"), "ジェネレーティブ デザイン");
        assert_eq!(tr("coming later"), "準備中");
        assert_eq!(tr("Sketches"), "スケッチ");
        assert_eq!(tr("NonExistentKey"), "NonExistentKey");

        assert_eq!(tr_dyn("Sketch1"), "スケッチ1");
        assert_eq!(tr_dyn("Body2"), "ボディ2");
        assert_eq!(tr_dyn("Units: mm"), "単位: mm");
    }

    #[test]
    fn keeps_english() {
        set_current_lang("en");
        assert_eq!(tr("Preferences"), "Preferences");
        assert_eq!(tr("Extrude"), "Extrude");
        assert_eq!(tr_dyn("Sketch1"), "Sketch1");
    }
}
