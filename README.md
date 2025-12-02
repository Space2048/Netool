# Netool 打包与使用文档

本文档详细说明如何编译、打包和使用 `netool` 网络测试工具。

## 1. 编译与打包 (Build & Package)

### 环境要求
- Rust 工具链 (Cargo, Rustc)
- 网络连接 (用于下载依赖)

### 1.1 Linux 编译

#### 通用编译 (动态链接)
适用于目标机器与编译机器系统版本相近的情况。
```bash
cargo build --release
```
产物路径:
- [target/release/netool-server](file:///f:/code/netool/target/release/netool-server)
- [target/release/netool-client](file:///f:/code/netool/target/release/netool-client)

#### 静态编译 (推荐)
适用于目标机器 Linux 发行版较老 (如 CentOS 7) 或为了最大兼容性。解决 `GLIBC_x.xx not found` 错误。

1. 安装 Musl target:
   ```bash
   rustup target add x86_64-unknown-linux-musl
   ```
2. 编译:
   ```bash
   cargo build --release --target x86_64-unknown-linux-musl
   ```
产物路径:
- [target/x86_64-unknown-linux-musl/release/netool-server](file:///f:/code/netool/target/x86_64-unknown-linux-musl/release/netool-server)
- [target/x86_64-unknown-linux-musl/release/netool-client](file:///f:/code/netool/target/x86_64-unknown-linux-musl/release/netool-client)

### 1.2 Windows 编译
```powershell
cargo build --release
```
产物路径:
- `target/release/netool-server.exe`
- `target/release/netool-client.exe`

### 1.3 打包清单
将以下文件复制到目标机器即可运行，无需安装其他依赖：

**Server 端:**
- `netool-server` (或 `.exe`)

**Client 端:**
- `netool-client` (或 `.exe`)
- `static/` 目录 (Web 界面资源)

### 1.4 一键打包 (One-click Packaging)
项目中提供了自动打包脚本，可自动编译并收集所有必要文件到 `dist` 目录。

**Windows:**
```powershell
./package.ps1
```

**Linux/macOS:**
```bash
chmod +x package.sh
./package.sh
```

---

## 2. 使用说明 (Usage)

### 2.1 服务端 (Server)
运行在被测机器上，负责响应客户端的测试请求。

**启动命令:**
```bash
# 默认监听 8080 端口
./netool-server

# 指定端口
./netool-server --port 9000
```

### 2.2 客户端 (Client)
运行在任意机器上，用于发起测试。

**基本语法:**
```bash
./netool-client --target <SERVER_IP>:<PORT> <MODE> [ARGS]
```

#### 功能 1: 连通性测试 (Ping)
测试能否连接到服务器控制端口，并计算往返延迟 (RTT)。
```bash
./netool-client --target 192.168.1.100:8080 mode ping
```

#### 功能 2: 带宽测速 (Speed Test)
测试从服务器下载数据的速度。
```bash
# 默认测试 10 秒
./netool-client --target 192.168.1.100:8080 mode speed

# 指定测试时长 (例如 30 秒)
./netool-client --target 192.168.1.100:8080 mode speed --duration 30
```

#### 功能 3: 端口连通性测试 (Port Test)
请求服务器打开指定端口，并验证客户端能否连接。
*注意: 采用按需打开策略，测试完成后服务器会自动关闭端口。*

```bash
# 测试连续端口范围
./netool-client --target 192.168.1.100:8080 mode ports --range 9000-9005

# 测试特定端口列表
./netool-client --target 192.168.1.100:8080 mode ports --range 80,443,8080,9000-9100
```
**输出示例:**
```text
Requesting to open 100 ports...
Server opened 100 ports. Testing connectivity...
Progress: 0/100 ports checked (0.0%)
Progress: 20/100 ports checked (20.0%)
...
Test complete. Success: 100, Failed: 0
Closing ports...
```

#### 功能 4: Web 界面 (Web UI)
启动一个 Web 服务器，提供可视化的测试界面。
```bash
# 默认监听 127.0.0.1:3000
./netool-client --target 192.168.1.100:8080 mode web

# 指定监听地址
./netool-client --target 192.168.1.100:8080 mode web --listen 0.0.0.0:3000
```
访问浏览器 `http://localhost:3000` (或指定地址) 即可使用 Ping、Speed Test 和 Port Test 功能。
*注意: 运行命令时，当前目录下必须包含 `static` 文件夹，否则页面将无法加载 (404 Not Found)。*

## 3. 常见问题 (Troubleshooting)

**Q: 运行提示 `/lib64/libm.so.6: version 'GLIBC_2.29' not found`**
A: 目标系统 glibc 版本过低。请参考 **1.1 Linux 编译 -> 静态编译** 章节，使用 `musl` 重新编译。

**Q: 端口测试全部失败 (Failed)**
A: 请检查服务器防火墙 (Firewall/iptables) 是否允许了测试端口范围的入站连接。
   - 云服务器: 检查安全组规则。
   - 本地 Linux: `ufw allow 9000:9100/tcp` 或 `iptables` 设置。

**Q: 使用 `cargo run` 报错 `Error loading target specification`**
A: 这是因为 `cargo` 也有 `--target` 参数 (用于交叉编译)。如果直接运行 `cargo run --target ...`，Cargo 会以为你在指定编译目标。
   **解决方法:** 在 `cargo run` 和参数之间加上 `--` 分隔符：
   ```bash
   cargo run --bin netool-client -- --target 192.168.1.100:8080 mode ping
   ```
