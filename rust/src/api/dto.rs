//! 跨语言传输的纯数据类型（都是「镜像结构体」，只有字段）。

/// 一条任务：选 / 退 / 查某一门课。
#[derive(Clone, Debug, Default)]
pub struct TaskView {
    /// 0 = 必修，1 = 选修
    pub category: u8,
    /// add / del / query
    pub action: String,
    pub kch: String,
    pub kxh: String,
    pub kcm: String,
}

/// `conf.json` 在界面上的样子。账号密码是用户自己填在本机文件里的。
#[derive(Clone, Debug, Default)]
pub struct ConfView {
    pub path: String,
    /// 这个路径上是不是真有一份配置文件
    pub exists: bool,
    pub loginname: String,
    pub password: String,
    pub batch: String,
    pub ocr_captcha: bool,
    pub always: bool,
    pub interval_ms: u64,
    pub tasks: Vec<TaskView>,
    /// 额外说明，例如「配置文件不存在，已用默认值」
    pub note: String,
}

/// 一次验证码。
#[derive(Clone, Debug, Default)]
pub struct CaptchaView {
    pub png: Vec<u8>,
    pub uuid: String,
    /// 自动识别读到的字符（手动模式为空）
    pub ocr_text: String,
}

/// 一个选课批次。
#[derive(Clone, Debug, Default)]
pub struct BatchView {
    pub code: String,
    pub name: String,
    pub open: bool,
}

/// 登录结果。
#[derive(Clone, Debug, Default)]
pub struct LoginView {
    pub loginname: String,
    pub message: String,
    pub batches: Vec<BatchView>,
}

/// 课程列表里的一行：一门课的一个教学班。
#[derive(Clone, Debug, Default)]
pub struct CourseRow {
    pub no: u32,
    pub kch: String,
    pub kcm: String,
    pub kxh: String,
    pub class_type: String,
    pub clazz_id: String,
    pub secret: String,
    pub teacher: String,
    pub time: String,
    pub place: String,
    pub credit: String,
    pub capacity: i64,
    pub selected: i64,
    pub available: bool,
    /// 原始 JSON，界面上放在「原始数据」里
    pub raw: String,
}

/// 一次选课 / 退课的结果。
#[derive(Clone, Debug, Default)]
pub struct SubmitResult {
    pub kch: String,
    pub kxh: String,
    pub action: String,
    pub ok: bool,
    pub message: String,
}
