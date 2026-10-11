# PC / Android 共用云端 API 配置

将 Android 的完整 JSON v1 文件原样复制到播放器可执行文件旁 `config/cloud_api/`，随后在「音乐联网搜索」点击「重读配置」。也可以在设置 → 联网服务选择其他 API 目录。读取目录中所有 `.json` 文件，按文件名排序，合计最多16个 provider，id不可重复。没有 JSON 时使用编译内置的同一份 GD 示例；任何文件验证失败时保留上次有效配置。

`example-gd.json` 与本次 Android 源码的 `online-providers.json` 内容相同，包含来源、五档请求音质、封面与翻译歌词操作。GD 的出处与用途字段保留在 JSON 中。存在其他 JSON 时，`example-gd.json` 作为回退，不参与合并；需要同时启用它时可另命名，仍须保持 provider id 唯一；不要把凭据提交到 Git。

配置说明见 [联网功能文档](../../docs/ONLINE_API.md)。默认歌曲与歌词两个开关均关闭，设置中分别开启。
