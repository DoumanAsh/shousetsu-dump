use shousetsu_dump::{novel, http};
use shousetsu_dump::utils::StrExt;

#[test]
fn should_split_exact() {
    let kakuyomu_url = "https://kakuyomu.jp/works/<id>";
    let syosetu_url = "https://ncode.syosetu.com/<id>";

    let result = kakuyomu_url.split_exact_by::<5>('/').expect("to split");
    assert_eq!(result, ["https:", "", "kakuyomu.jp", "works", "<id>"]);
    let result = syosetu_url.split_exact_by::<4>('/').expect("to split");
    assert_eq!(result, ["https:", "", "ncode.syosetu.com", "<id>"]);
    let result = syosetu_url.split_exact_by::<3>('/').expect("to split");
    assert_eq!(result, ["https:", "", "ncode.syosetu.com/<id>"]);
    assert!(syosetu_url.split_exact_by::<5>('/').is_none());
}

#[test]
fn should_rsplit_exact() {
    let kakuyomu_url = "https://kakuyomu.jp/works/<id>";
    let syosetu_url = "https://ncode.syosetu.com/<id>";

    let result = kakuyomu_url.rsplit_exact_by::<5>('/').expect("to split");
    assert_eq!(result, ["<id>", "works", "kakuyomu.jp", "", "https:"]);

    let result = syosetu_url.rsplit_exact_by::<4>('/').expect("to split");
    assert_eq!(result, ["<id>", "ncode.syosetu.com", "", "https:"]);
    let result = syosetu_url.rsplit_exact_by::<3>('/').expect("to split");
    assert_eq!(result, ["<id>", "ncode.syosetu.com", "https:/"]);
    let result = syosetu_url.rsplit_exact_by::<2>('/').expect("to split");
    assert_eq!(result, ["<id>", "https://ncode.syosetu.com"]);

    let result = kakuyomu_url.rsplit_exact_by::<3>('/').expect("to split");
    assert_eq!(result, ["<id>", "works", "https://kakuyomu.jp"]);
    let result = kakuyomu_url.rsplit_exact_by::<2>('/').expect("to split");
    assert_eq!(result, ["<id>", "https://kakuyomu.jp/works"]);

    assert!(syosetu_url.rsplit_exact_by::<5>('/').is_none());
}

#[test]
fn should_parse_novel_id() {
    let kakuyomu_url = "https://kakuyomu.jp/works/1177354054935164320";
    let syosetu_url = "https://ncode.syosetu.com/n8856gp/";
    let r18syosetu_url = "https://novel18.syosetu.com/n9598df/";

    let id = novel::Id::try_parse(kakuyomu_url).expect("parse kakuyomu_url");
    assert_eq!(id.kind(), novel::BackendKind::Kakuyomu);
    assert!(id.kind().is_kakuyomu());
    assert_eq!(id.id(), "1177354054935164320");

    let id = novel::Id::try_parse(syosetu_url).expect("parse syosetu_url");
    assert_eq!(id.kind(), novel::BackendKind::Syosetu);
    assert!(id.kind().is_syosetu());
    assert_eq!(id.id(), "n8856gp");

    let id = novel::Id::try_parse(r18syosetu_url).expect("parse r18syosetu_url");
    assert_eq!(id.kind(), novel::BackendKind::R18Syosetu);
    assert!(id.kind().is_syosetu());
    assert_eq!(id.id(), "n9598df");
}

#[test]
fn should_parse_kakuyomu_title() {
    const DATA: &str = r#"超難関ダンジョンで10万年修行した結果、世界最強に～最弱無能の下剋上～（最弱で迫害までされていたけど、超難関迷宮で10万年修行した結果、強くなりすぎて敵がいなくなった）（力水） - カクヨム"#;

    let title = novel::Title::new_kakuyomu(DATA);
    assert_eq!(title.name, "超難関ダンジョンで10万年修行した結果、世界最強に～最弱無能の下剋上～（最弱で迫害までされていたけど、超難関迷宮で10万年修行した結果、強くなりすぎて敵がいなくなった）");
    assert_eq!(title.author, Some("力水"));
}

#[test]
fn should_parse_empty_syosetu_result() {
    const DATA: &str = r#"[{"allcount":0}]"#;

    let result: Vec<shousetsu_dump::novel::syosetu::ApiResponse> = serde_json::from_str(DATA).expect("to parse empty result");
    assert_eq!(result.len(), 1);
}

#[test]
fn should_parse_valid_syosetu_result() {
    const DATA: &str = r#"[{"allcount":1},{"title":"\u971c\u6708\u3055\u3093\u306f\u30e2\u30d6\u304c\u597d\u304d","ncode":"N8856GP","userid":866237,"writer":"\u516b\u795e\u93e1@\u5e7c\u5973\u66f8\u7c4d\u5316\uff06\u300e\u971c\u6708\u3055\u3093\u306f\u30e2\u30d6\u304c\u597d\u304d\u300f5\u5dfb","story":"\u3010\u30b3\u30df\u30ab\u30e9\u30a4\u30ba\uff06\u66f8\u7c4d\u51685\u5dfb\u767a\u58f2\u4e2d\uff01\u3011\/\/\u5b8c\u7d50\u3057\u307e\u3057\u305f\uff01\/\/\n\u666e\u901a\u306e\u30e9\u30d6\u30b3\u30e1\u306a\u3089\u3001\u4ffa\u306f\u305f\u3060\u306e\u30e2\u30d6\u30ad\u30e3\u30e9\u3067\u3057\u304b\u306a\u304b\u3063\u305f\u3060\u308d\u3046\u3002\u7fa9\u7406\u306e\u59b9\u3082\u3001\u5973\u53cb\u9054\u3082\u3001\u5e7c\u99b4\u67d3\u3082\u3001\u307f\u3093\u306a\u30e2\u30c6\u30e2\u30c6\u306a\u3042\u3044\u3064\u3092\u597d\u304d\u306b\u306a\u3063\u305f\u3002\u4f55\u3082\u305b\u305a\u3068\u3082\u751f\u307e\u308c\u306a\u304c\u3089\u306b\u5973\u5b50\u306b\u597d\u304b\u308c\u308b\u4e3b\u4eba\u516c\u69d8\u306f\u3001\u5446\u6c17\u306a\u304f\u4ffa\u304c\u5927\u5207\u306b\u601d\u3063\u3066\u3044\u305f\u5f7c\u5973\u305f\u3061\u3092\u30cf\u30fc\u30ec\u30e0\u30e1\u30f3\u30d0\u30fc\u306b\u3057\u3066\u3001\u9752\u6625\u30e9\u30d6\u30b3\u30e1\u3092\u697d\u3057\u3093\u3067\u3044\u308b\u3002\u30e2\u30d6\u30ad\u30e3\u30e9\u306e\u4ffa\u306f\u3001\u6559\u5ba4\u306e\u7aef\u3063\u3053\u304b\u3089\u4e3b\u4eba\u516c\u69d8\u3092\u773a\u3081\u308b\u3053\u3068\u3057\u304b\u3067\u304d\u306a\u3044\u306f\u305a\u3060\u3063\u305f\u2026\u2026\u3067\u3082\u3001\u5f7c\u5973\u306f\u305d\u3093\u306a\u4ffa\u3092\u898b\u3064\u3051\u3066\u304f\u308c\u305f\u3002\u4e3b\u4eba\u516c\u69d8\u306e\u5e7c\u99b4\u67d3\u3067\u3042\u308a\u3001\u30e1\u30a4\u30f3\u30d2\u30ed\u30a4\u30f3\u3068\u3044\u3046\u7acb\u5834\u306b\u3044\u308b\u306b\u3082\u95a2\u308f\u3089\u305a\u3001\u5f7c\u5973\u306f\u4ffa\u3092\u9078\u3093\u3067\u304f\u308c\u305f\u3002\u666e\u6bb5\u306f\u7121\u53e3\u3067\u7121\u8868\u60c5\u3060\u3051\u3069\u3001\u5f7c\u5973\u306f\u4ffa\u306b\u3060\u3051\u306f\u30aa\u30b7\u30e3\u30d9\u30ea\u306b\u306a\u308b\u3057\u3001\u7b11\u9854\u3092\u898b\u305b\u3066\u304f\u308c\u308b\u3002\u4e3b\u4eba\u516c\u69d8\u3088\u308a\u3082\u3001\u30e2\u30d6\u30ad\u30e3\u30e9\u306e\u4ffa\u3092\u7279\u5225\u3060\u3068\u8a00\u3063\u3066\u304f\u308c\u305f\u306e\u3060\u2015\u2015\u3053\u308c\u306f\u3001\u305d\u3093\u306a\u51b4\u3048\u306a\u3044\u30e2\u30d6\u30ad\u30e3\u30e9\u306e\u7269\u8a9e\u3002\u30e1\u30bf\u8996\u70b9\u3067\u7269\u8a9e\u308b\u3001\u971c\u6708\u3055\u3093\u304c\u597d\u304d\u306b\u306a\u3063\u3066\u304f\u308c\u305f\u30e2\u30d6\u30ad\u30e3\u30e9\u306e\u30e9\u30d6\u30b3\u30e1\u3067\u3042\u308b\u3002\n\n\n\/\/\u4f5c\u54c1\u60c5\u5831\/\/\n\u7b2c\u4e5d\u56de\u30cd\u30c3\u30c8\u5c0f\u8aac\u5927\u8cde\u671f\u9593\u4e2d\u53d7\u8cde\u3002\u66f8\u7c4d\u306f\u5b8c\u7d50\u3057\u307e\u3057\u305f\uff01(\u51685\u5dfb)\n\u30a4\u30e9\u30b9\u30c8\u306fRoha\u5148\u751f\u3002\u30de\u30a4\u30af\u30ed\u30de\u30ac\u30b8\u30f3\u793e\uff08GCN\u6587\u5eab\uff09\u69d8\u3088\u308a\u767a\u58f2\u4e2d\n\u30b3\u30df\u30c3\u30af\u30e9\u30a4\u30c9\u69d8\u3088\u308a\u3001\u30b3\u30df\u30ab\u30e9\u30a4\u30ba\u7b2c\u4e00\u8a71\u304c\u516c\u958b\u3055\u308c\u3066\u304a\u308a\u307e\u3059\uff01\n\u30b3\u30df\u30c3\u30af2\u5dfb\u767a\u58f2\u4e2d\u3000\u66f8\u7c4d\u306f5\u5dfb\u307e\u3067\u767a\u58f2\u4e2d\u3000CM\u3082\u516c\u958b\u4e2d\n\u65e7\u984c\u300e\u30e2\u30c6\u30e2\u30c6\u30cf\u30fc\u30ec\u30e0\u4e3b\u4eba\u516c\u69d8\u304c\u611b\u3057\u3066\u3044\u308b\u5e7c\u99b4\u67d3\u306e\u7121\u53e3\u30d2\u30ed\u30a4\u30f3\u304c\u30e2\u30d6\u30ad\u30e3\u30e9\u306e\u4ffa\u306b\u3060\u3051\u30aa\u30b7\u30e3\u30d9\u30ea\u3067\u53ef\u611b\u3044\u3093\u3060\u304c\u300f\u203b\u30ab\u30af\u30e8\u30e0\u306b\u3082\u6295\u7a3f\u3057\u3066\u304a\u308a\u307e\u3059\u3002\n\u3010\u795d\u3011\u3010\u66f8\u7c4d\u5316\u3011\u3010\u30b3\u30df\u30ab\u30e9\u30a4\u30ba\u3011\u8a55\u4fa1pt72000\u7a81\u7834\u3001\u30d6\u30af\u30de18000\u4ef6\u7a81\u7834\u3001\u611f\u60f32500\u4ef6\u7a81\u7834\u3001\u73fe\u5b9f\u4e16\u754c\u3014\u604b\u611b\u3015\u30e9\u30f3\u30ad\u30f3\u30b01\u4f4d\uff08\u6700\u9ad8\uff09","biggenre":1,"genre":102,"gensaku":"","keyword":"\u30cf\u30fc\u30ec\u30e0 \u9752\u6625 \u30b9\u30af\u30fc\u30eb\u30e9\u30d6 \u30e9\u30d6\u30b3\u30e1 \u3056\u307e\u3041 \u30a2\u30f3\u30c1\u30cf\u30fc\u30ec\u30e0 \u30e4\u30f3\u30c7\u30ec \u5e7c\u99b4\u67d3\u307f \u30cd\u30c3\u30c8\u5c0f\u8aac\u5927\u8cde\u4e5d \u66f8\u7c4d\u5316 GCN\u6587\u5eab \u30b3\u30df\u30ab\u30e9\u30a4\u30ba \u66f8\u7c4d\u5b8c\u7d50 web\u7248\u5b8c\u7d50","general_firstup":"2020-11-21 20:12:27","general_lastup":"2025-08-15 21:04:21","novel_type":1,"end":1,"general_all_no":654,"length":1096639,"time":2194,"isstop":1,"isr15":0,"isbl":0,"isgl":0,"iszankoku":0,"istensei":0,"istenni":0,"global_point":76170,"daily_point":2,"weekly_point":30,"monthly_point":108,"quarter_point":372,"yearly_point":2374,"fav_novel_cnt":19058,"impression_cnt":2603,"review_cnt":26,"all_point":38054,"all_hyoka_cnt":4210,"sasie_cnt":11,"kaiwaritu":27,"novelupdated_at":"2025-12-09 06:13:02","updated_at":"2026-02-14 11:52:02"}]"#;

    let result: Vec<shousetsu_dump::novel::syosetu::ApiResponse> = serde_json::from_str(DATA).expect("to parse empty result");
    assert_eq!(result.len(), 2);
}

#[test]
fn should_validate_url_redirect_resolver() {
    let http = http::Client::new();
    let url = http.resolve_url_location("https://21028.mitemin.net/userpageimage/viewimagebig/icode/i581566/");
    assert_eq!(url, "https://img1.mitemin.net/iz/ux/jtdmiq9jj92djjdogu5c7jo14i9f_jk7_yd_1ci_132jk.jpg.580.jpg");
}
