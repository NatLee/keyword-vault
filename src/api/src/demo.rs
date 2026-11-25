//! 範例資料：`keyword-vault demo` 在空的資料庫裡建立幾個帳號、群組和關鍵字，
//! 用來試用或截圖。內容都是虛構的，時間以執行當下往回推。

use anyhow::{anyhow, bail, Result};
use chrono::{Duration, Utc};

use crate::{
    auth, db,
    routes::keywords::{self, NewKeyword},
    state::AppState,
};

pub const MEMBER_USERNAMES: [&str; 3] = ["aki", "yiting", "kevin"];

const USERS: [(&str, &str, &str); 4] = [
    ("demo", "林小安", "admin"),
    ("aki", "阿哲", "member"),
    ("yiting", "怡婷", "member"),
    ("kevin", "凱文", "member"),
];

/// 群組名稱和成員（`USERS` 的位置）
const GROUPS: [(&str, &[usize]); 2] = [("行銷企劃", &[0, 1, 2]), ("產品研究", &[0, 3])];

struct Seed {
    owner: usize,
    theme: &'static str,
    /// 一般關鍵字是語法；組合關鍵字是組合方式，`{n}` 代表這張表的第 n 個
    body: &'static str,
    combination: bool,
    visibility: &'static str,
    tags: &'static [&'static str],
    days_ago: i64,
}

const fn plain(
    owner: usize,
    theme: &'static str,
    body: &'static str,
    visibility: &'static str,
    tags: &'static [&'static str],
    days_ago: i64,
) -> Seed {
    Seed {
        owner,
        theme,
        body,
        combination: false,
        visibility,
        tags,
        days_ago,
    }
}

const fn combo(
    owner: usize,
    theme: &'static str,
    body: &'static str,
    visibility: &'static str,
    tags: &'static [&'static str],
    days_ago: i64,
) -> Seed {
    Seed {
        owner,
        theme,
        body,
        combination: true,
        visibility,
        tags,
        days_ago,
    }
}

#[rustfmt::skip]
const KEYWORDS: &[Seed] = &[
    /*  0 */ plain(0, "珍珠奶茶", "珍珠奶茶|波霸奶茶|黑糖珍珠|(珍珠 鮮奶 ~)", "public", &["飲料", "手搖飲"], 320),
    /*  1 */ plain(0, "手搖飲通稱", "手搖飲|手搖杯|飲料店|(手搖 飲料 ~5)", "public", &["飲料", "手搖飲"], 300),
    /*  2 */ plain(0, "排除：業配與抽獎", "業配|工商|抽獎|團購|(留言 抽 ~3)", "public", &["排除詞"], 290),
    /*  3 */ plain(1, "咖啡", "咖啡|拿鐵|美式|手沖|(精品 咖啡 ~)", "public", &["飲料"], 270),
    /*  4 */ plain(1, "連鎖咖啡店", "(咖啡&(連鎖|門市|展店))|咖啡廳", "group", &["飲料", "通路"], 255),
    /*  5 */ plain(2, "外送平台", "外送平台|外送員|(美食 外送 ~10)|(外送&(訂餐|運費))", "public", &["生活", "通路"], 240),
    /*  6 */ plain(3, "電動車", "電動車|EV|(電動 汽車 ~)|充電樁|(續航&里程)", "public", &["汽車"], 230),
    /*  7 */ plain(3, "電動機車", "電動機車|(電動 機車 ~)|換電站|(電池&(交換|月租))", "public", &["汽車"], 215),
    /*  8 */ plain(3, "排除：二手與租賃", "二手|中古|租賃|(長租 方案 ~)", "public", &["排除詞", "汽車"], 200),
    /*  9 */ plain(2, "露營", "露營|露營區|(免裝備 露營 ~)|豪華露營|車泊", "public", &["戶外", "旅遊"], 190),
    /* 10 */ plain(2, "登山", "登山|百岳|郊山|(入山 申請 ~10)|山屋", "public", &["戶外"], 175),
    /* 11 */ plain(0, "日本旅遊", "(日本&(旅遊|自由行|機票))|東京|大阪|北海道|沖繩", "public", &["旅遊"], 160),
    /* 12 */ plain(0, "韓國旅遊", "(韓國&(旅遊|自由行))|首爾|釜山|濟州島", "public", &["旅遊"], 150),
    /* 13 */ plain(1, "廉價航空", "廉航|廉價航空|(促銷 機票 ~10)|(機票&(特價|開賣))", "group", &["旅遊"], 140),
    /* 14 */ plain(2, "串流影音", "串流|(影音 平台 ~5)|追劇|(訂閱&(漲價|方案))", "public", &["娛樂"], 130),
    /* 15 */ plain(2, "韓劇", "韓劇|(韓國 影集 ~)|(歐巴&追劇)", "private", &["娛樂"], 120),
    /* 16 */ plain(3, "智慧型手機", "手機|智慧型手機|(旗艦 機 ~3)|(摺疊&(手機|螢幕))", "public", &["3C"], 110),
    /* 17 */ plain(3, "手機相機", "(手機&(相機|鏡頭|夜拍|望遠))|(拍照 手機 ~5)", "public", &["3C"], 100),
    /* 18 */ plain(3, "筆電", "筆電|筆記型電腦|(輕薄 筆電 ~)|(電競&筆電)", "group", &["3C"], 92),
    /* 19 */ plain(1, "信用卡回饋", "信用卡&(回饋|現金回饋|點數|哩程)", "public", &["金融"], 85),
    /* 20 */ plain(1, "行動支付", "行動支付|電子支付|(掃碼 付款 ~5)|(錢包&(綁定|儲值))", "public", &["金融"], 78),
    /* 21 */ plain(0, "颱風假", "颱風假|(停班 停課 ~)|(颱風&(放假|停班))", "public", &["時事"], 70),
    /* 22 */ plain(0, "地震", "地震|(有感 地震 ~)|(震度&(規模|芮氏))|餘震", "private", &["時事"], 64),
    /* 23 */ plain(2, "健身房", "健身房|重訓|(私人 教練 ~)|(健身&(會籍|月費))", "public", &["運動"], 58),
    /* 24 */ plain(2, "路跑", "路跑|馬拉松|半馬|(完賽 獎牌 ~10)", "public", &["運動"], 50),
    /* 25 */ plain(1, "寵物", "寵物|毛小孩|(貓 飼料 ~5)|(狗 飼料 ~5)|(寵物&(保險|友善))", "public", &["生活"], 44),
    /* 26 */ plain(3, "遊戲主機", "遊戲主機|(家用 主機 ~)|(主機&(獨佔|首發))|掌機", "public", &["3C", "娛樂"], 37),
    /* 27 */ plain(0, "租屋", "租屋|(套房 出租 ~5)|(房東&(押金|租約))|租金補貼", "group", &["生活", "居住"], 30),
    /* 28 */ plain(1, "保健食品", "保健食品|益生菌|葉黃素|魚油|(維他命&(推薦|比較))", "public", &["健康"], 24),
    /* 29 */ plain(2, "無糖茶", "無糖茶|(無糖 綠茶 ~)|(無糖&(烏龍|青茶))", "public", &["飲料"], 17),
    /* 30 */ plain(0, "排除：徵才", "徵才|徵人|工讀|(門市 人員 ~)|時薪", "public", &["排除詞"], 12),
    /* 31 */ plain(3, "智慧手錶", "智慧手錶|(運動 手錶 ~)|(手錶&(心率|血氧))", "private", &["3C", "運動"], 6),
    /* 32 */ combo(0, "手搖飲聲量（去業配）", "({0}|{1})!{2}", "public", &["飲料", "手搖飲"], 280),
    /* 33 */ combo(0, "飲料全品類", "{0}|{1}|{3}|{29}", "public", &["飲料"], 15),
    /* 34 */ combo(0, "飲料全品類（去雜訊）", "{33}!({2}|{30})", "public", &["飲料"], 10),
    /* 35 */ combo(3, "電動載具（排除二手）", "({6}|{7})!{8}", "group", &["汽車"], 195),
    /* 36 */ combo(2, "戶外活動", "{9}|{10}|{24}", "public", &["戶外"], 45),
    /* 37 */ combo(0, "東北亞旅遊", "{11}|{12}", "public", &["旅遊"], 145),
    /* 38 */ combo(3, "手機與穿戴", "{16}|{17}|{31}", "private", &["3C"], 5),
];

/// 修改紀錄：（關鍵字、改之前的語法、誰改的、幾天前）
const HISTORY: &[(usize, &str, usize, i64)] = &[
    (0, "珍珠奶茶|波霸奶茶", 0, 260),
    (0, "珍珠奶茶|波霸奶茶|黑糖珍珠", 0, 120),
    (6, "電動車|EV|(電動 汽車 ~)", 3, 150),
    (16, "手機|智慧型手機", 3, 60),
    (21, "颱風假|(停班 停課 ~)", 0, 21),
];

/// 留言：（關鍵字、誰、內容、幾天前）
const COMMENTS: &[(usize, usize, &str, i64)] = &[
    (
        0,
        1,
        "「黑糖珍珠」會不會抓到甜點的文章？要不要加排除詞",
        119,
    ),
    (
        0,
        0,
        "有道理，我在「手搖飲聲量（去業配）」那個組合裡處理",
        118,
    ),
    (
        6,
        2,
        "EV 這個詞在英文文章裡會抓到很多無關的，建議搭配其他條件",
        100,
    ),
    (32, 3, "這組拿去跑上個月的報表，結果比之前乾淨很多", 30),
    (11, 1, "要不要把九州、福岡也加進來？", 20),
    (21, 2, "颱風季快到了，先收藏", 8),
];

/// 收藏：（誰、關鍵字）
const FAVORITES: &[(usize, usize)] = &[
    (0, 3),
    (0, 6),
    (0, 9),
    (0, 14),
    (0, 36),
    (1, 0),
    (1, 32),
    (2, 0),
    (2, 11),
    (2, 21),
    (3, 32),
];

pub struct Summary {
    pub users: usize,
    pub keywords: usize,
}

fn days_ago(days: i64) -> String {
    // 每筆錯開幾個小時，列表看起來才不會都在同一個時間
    db::format_time(Utc::now() - Duration::days(days) - Duration::hours(days % 7))
}

pub async fn seed(state: &AppState, password: &str) -> Result<Summary> {
    let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await?;
    if existing > 0 {
        bail!("資料庫裡已經有帳號了；範例資料只能建在空的資料庫");
    }
    let app_err = |e: crate::error::AppError| anyhow!("{e:?}");

    let mut user_ids = Vec::new();
    for (username, display_name, role) in USERS {
        let id = auth::create_user(&state.db, username, display_name, password, role)
            .await
            .map_err(app_err)?;
        user_ids.push(id);
    }

    let mut tx = state.db.begin().await?;
    for (name, members) in GROUPS {
        let group_id = sqlx::query("INSERT INTO user_groups (name, created_at) VALUES (?, ?)")
            .bind(name)
            .bind(days_ago(330))
            .execute(&mut *tx)
            .await?
            .last_insert_rowid();
        for member in members {
            sqlx::query("INSERT INTO group_members (group_id, user_id) VALUES (?, ?)")
                .bind(group_id)
                .bind(user_ids[*member])
                .execute(&mut *tx)
                .await?;
        }
    }

    let mut ids: Vec<i64> = Vec::new();
    for seed in KEYWORDS {
        let (expression, logic) = if seed.combination {
            // `{n}` 換成第 n 個關鍵字實際的編號
            let mut logic = seed.body.to_owned();
            for (n, id) in ids.iter().enumerate().rev() {
                logic = logic.replace(&format!("{{{n}}}"), &format!("#{id}"));
            }
            (None, Some(logic))
        } else {
            (Some(seed.body.to_owned()), None)
        };
        let new = NewKeyword {
            theme: seed.theme.to_owned(),
            expression,
            logic,
            visibility: seed.visibility.to_owned(),
            tags: seed.tags.iter().map(|t| t.to_string()).collect(),
        };
        let id = keywords::insert(
            &mut tx,
            user_ids[seed.owner],
            state.config.default_slop,
            new,
            &days_ago(seed.days_ago),
        )
        .await
        .map_err(|e| anyhow!("範例關鍵字「{}」建不起來：{e:?}", seed.theme))?;
        ids.push(id);
    }

    for (keyword, expression, user, days) in HISTORY {
        let at = days_ago(*days);
        sqlx::query(
            "INSERT INTO history (keyword_id, expression, user_id, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(ids[*keyword])
        .bind(expression)
        .bind(user_ids[*user])
        .bind(&at)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE keywords SET updated_at = MAX(updated_at, ?) WHERE id = ?")
            .bind(&at)
            .bind(ids[*keyword])
            .execute(&mut *tx)
            .await?;
    }
    for (keyword, user, body, days) in COMMENTS {
        let at = days_ago(*days);
        sqlx::query(
            "INSERT INTO comments (keyword_id, user_id, body, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(ids[*keyword])
        .bind(user_ids[*user])
        .bind(body)
        .bind(&at)
        .bind(&at)
        .execute(&mut *tx)
        .await?;
    }
    for (user, keyword) in FAVORITES {
        sqlx::query("INSERT INTO favorites (user_id, keyword_id, created_at) VALUES (?, ?, ?)")
            .bind(user_ids[*user])
            .bind(ids[*keyword])
            .bind(db::now())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    Ok(Summary {
        users: user_ids.len(),
        keywords: ids.len(),
    })
}
