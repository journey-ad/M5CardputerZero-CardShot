# CardShot

Cardputer Zero（aarch64 Linux，320×170 帧缓冲）上的后台截图 / 录屏 / 相册管理工具。

由三部分组成：

- `rust/cardshotd`：常驻后台守护，读全局 `/dev/fb0` 抓取任意前台应用画面，支持截图、录屏、相册、Unix socket 控制与 HTTP 远程管理
- `rust/cardshot-ui`：设备端界面（Rust + Slint），相册管理工具：浏览、预览、多选删除、设置
- `service/web`：远程 Web 管理页（Preact + htm，无构建步骤）

## 目录

```
rust/
  cardshot-core/   配置、抓帧、编码、相册、录制
  cardshotd/       守护进程：控制 socket、热键、ACT LED、HTTP 远程
  cardshot-ui/     设备端 Slint 界面
service/
  cardshot.service   systemd 用户服务
  config.default.json 默认配置
  web/              Web 管理页与本地化依赖
design/             HTML 设计原型与图标、字体
scripts/            连接设备、构建部署、控制客户端
docs/feasibility.md 可行性分析
```

## 构建

守护与界面均为 aarch64 目标，用设备构建镜像交叉编译：

```sh
# 构建镜像（一次即可）
docker build --platform linux/arm64 -t cardbili-device-builder -f scripts/device.Dockerfile scripts

# 编译
docker run --rm --platform linux/arm64 -v "$PWD:/work" -v cardbili-device-cargo:/usr/local/cargo \
  -e CARGO_TARGET_DIR=/work/rust/target -w /work/rust cardbili-device-builder \
  bash -c 'export PATH=/usr/local/cargo/bin:$PATH; cargo build --release --target aarch64-unknown-linux-gnu'
```

## 部署

```sh
bash scripts/deploy-rust.sh
```

脚本会停止旧服务、上传二进制与 Web 资源、重载并重启 systemd 用户服务 `cardshot.service`。

## 使用

- 截图/录制：全局热键（默认关闭，可在设置里开启并配置截图快捷键），或在 Web 管理页操作
- 设备端界面：`cardshot-ui`，主页进入「相册」浏览，`Enter` 预览，`Space` 多选，`D` 删除，`Esc` 返回
- 远程管理：设备与电脑在同一局域网时访问 `http://<设备IP>:8420/`

## 配置

配置文件默认 `~/.config/cardshot/config.json`，首次运行会生成。主要项：

- `image_format`：`png` / `jpeg` / `webp`
- `image_quality`：1–100
- `record.fps`、`record.bitrate`、`record.encoder`
- `remote.enabled`、`remote.bind`、`remote.port`
- `hotkeys.enabled`、`hotkeys.shot`、`hotkeys.record`

## 控制接口

Unix socket（`$XDG_RUNTIME_DIR/cardshot.sock`）按行收发 JSON，命令包括：
`status` `shot` `record_start` `record_stop` `record_toggle` `items` `counts` `delete` `config_get` `config_set` `reload` `remote_start` `remote_stop` `remote_status` `thumb`

HTTP API：
`GET /api/status` `/api/items` `/api/config` `/api/thumb?id=` `/api/file?id=&download=1`
`POST /api/shot` `/api/record` `/api/delete` `/api/config` `/api/remote`
