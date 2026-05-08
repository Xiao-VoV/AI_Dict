#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    #[error("词典数据库错误：{0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("词典 JSON 错误：{0}")]
    Json(#[from] serde_json::Error),
    #[error("词典文件读写错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("词典路径错误：{0}")]
    Tauri(#[from] tauri::Error),
    #[error("内置 ECDICT SQLite 未安装，请将 ecdict.db 放入应用资源目录")]
    MissingBuiltinDb,
    #[error("内置词典不能删除")]
    CannotDeleteBuiltin,
    #[error("词典索引任务正在运行：{0}")]
    IndexTaskAlreadyRunning(String),
    #[error("MDX 词典解析错误：{0}")]
    Mdict(String),
}
