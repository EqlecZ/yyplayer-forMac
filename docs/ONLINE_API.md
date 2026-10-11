# 音乐联网搜索

从联网搜索进入设置后，可用侧栏「返回上一页」或 Alt+Left 返回原搜索页，保留关键词、API / 来源选择和已收到的结果。已有自定义 Alt+Left 播放快捷键优先保留。页面历史按本次会话逐层返回；重新启动后从音乐库开始。

左侧「音乐联网搜索」独立于音乐库和视频库。设置 → 联网服务分别开启歌曲与歌词联网（默认关闭），在页标题右侧输入歌名／歌手／专辑，按 Enter 或搜索按钮提交。结果列表滚到底部会自动追加下一页，保留当前滚动位置；结果不足一屏时自动补充，有请求进行中、查看历史或打开音质弹层时暂不翻页。末页、重复整页或请求失败后停止自动加载，失败保留已有结果并显示原因，可重新搜索。历史最多20条、最近优先、去空格、忽略大小写去重；「搜索历史」可查看、单条删除、清空或关闭记录。翻页和歌词候选不记录音乐历史。

选择 API 和来源后，结果显示真实封面、原始歌名／歌手／专辑。点击行按配置中的第一个适用音质串流播放，服务拒绝时显示原因，不偷偷降到另一档。右侧「…」提供播放／下载音质、逐档可用性检查、下载完成后播放。检查出的可用地址缓存60秒，最多128条，provider滚动窗口限流与Android一致。菜单中的音质和服务报告是请求结果；实际文件／输出状态取自文件头与现有 libmpv 快照。

串流复用现有音乐播放页、封面、歌词同步、暂停／seek／输出／EQ控件。开启歌词联网时自动按 lyricId取歌词；翻译按时间与原文合并。播放选项 → 联网获取歌词可独立为本地歌曲选择候选（歌曲联网关闭也可用），标题相同优先，可改关键词／API／来源。候选只属于当前歌曲，换歌或退出取消旧选择；手动关联优先于后续自动结果，普通本地／内嵌歌词读取保留。

下载先写 `.partial`，取消／失败清理未完成文件；完整文件按 `<歌名> - <歌手>.<真实格式>` 保存，重名加 `(2)` 等。文件头支持 FLAC、MP3、WAV、M4A、OGG、Opus、AAC、AIFF、APE，不按 URL、MIME、大小或请求档位猜扩展名。成功后加入音乐库，保留下载元数据关联，并尽力保存同名 LRC/JPG。辅助资源获取失败不阻止完整音频播放。关闭歌曲联网停止在线串流，已下载文件仍能播放；没有后台下载服务或断点续传。

设置中的五类目录独立：

| 目录 | 默认与行为 |
| --- | --- |
| API JSON | exe旁 `config/cloud_api`；可选其他目录，「打开文件夹」打开当前显示的目录（不存在时尝试创建），重读时验证全部文件，失败保留原配置 |
| 歌曲下载 | 用户 `Music/YYPlayer`；可选目录，不自动删除成品歌曲 |
| 歌词缓存 | 用户设置目录的 `cache/online-lyrics/yyplayer-lyrics`；自动缓存约16MiB，`selected` 手动关联保留 |
| 专辑图缓存 | `cache/online-artwork/yyplayer-artwork`；约128MiB，上限8MiB/8192边长，后台缩至1024，结果缩略图128px/32项 |
| 串流缓存 | `cache/online-stream/yyplayer-stream`；完整响应才复用，约1GiB，单曲512MiB；Range seek仍访问CDN |

缓存上限按写入后清理旧的应用文件计算，正在写入的少量有界任务可能暂时超过该数值。选择目录后在应用专属子目录存缓存，不移动已有文件；旧缓存可由用户自行处理。恢复默认仅改偏好。Linux 的下载原子发布要求目标文件系统支持 hard link；Windows 下载路径使用无覆盖原子重命名。

把 Android 的完整 JSON v1 原样放入 API 目录，可读多份文件（按名字排序，合计1–16个provider，id唯一）。不从Android工程绝对路径动态加载；发布源里的[GD示例](../config/cloud_api/example-gd.json)与本轮Android资产相同。没有JSON时使用同一份内置示例。存在其他JSON时，默认文件 `example-gd.json` 作为回退，不参与合并；若要同时启用GD与自定义源，可把示例另命名。带账号密钥的JSON只保存在用户所选目录，不进入设置、日志或播放历史；默认路径中的自定义 JSON 已加入 `.gitignore`。

| v1能力 | PC语义 |
| --- | --- |
| baseUrl / apiHosts / headers / mediaHeaders | HTTPS、API主机白名单、API与CDN头隔离；重定向换authority不携带原访问头 |
| search / stream / download / lyricSearch / lyric / artwork | 操作可独立配置，缺download复用stream，支持纯歌词provider |
| GET / POST、query / form / json、body | 参数UTF-8编码；嵌套body只替换字符串叶子，保留数组／数值／boolean／null |
| variables / trackFields / quality.parameters / quality.variables / quality.sources | 声明常量、曲目映射变量、逐档覆盖、来源限制；未知／重复保留变量拒绝 |
| response | 根数组／点分路径／数组索引、text、时长单位、qualities交集、url、quality、lyric、translation等 |
| limits | 配置256KiB/深度32、API响应2MiB/200条、2000条/32MiB分页结果、歌词1MB/10000行、歌曲512MiB |

`size/sizeUnit/format` 服务报告按映射读取并显示在音质检查中，不作为音频类型或完整传输的证明，下载核对真正Content-Length与收到的字节。GD保留三来源与128/192/320/740/999五档、40次/300秒限流，以及JSON的出处／CC BY-NC学习用途说明。可用性仍取决于服务与曲目供给。不支持运行JS、OAuth刷新、签名计算、加密歌词、DRM或承诺Android未实现的格式。

验证：Windows运行 `scripts/Test-Online.ps1`。它用现有 Python（Pillow/cryptography）与FFmpeg准备自有HTTPS服务，显式QA构建只信任本次localhost CA，使用独立target配置，验证真实libmpv播放，不改系统信任或APPDATA。完整结果及平台限制见[验收记录](validation/online-api.md)。
