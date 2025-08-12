
//初始化tk log
pub fn init_tk_log() {
    tklog::LOG.set_console(true)
        .set_level(tklog::LEVEL::Info)
        .set_formatter("{time} | {level} | {file} | {message}\n")
        .uselog();  // 启用官方log库
}