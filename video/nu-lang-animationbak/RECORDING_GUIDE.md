# 网页动画录制指南

将 Nu Language 动画页面录制成视频的多种方法。

## 🎥 方法1: Chrome DevTools (推荐，简单)

### 使用 Chrome 内置录屏功能

1. **打开动画页面**
   ```bash
   cd video/nu-lang-animation
   python -m http.server 8000
   # 访问 http://localhost:8000
   ```

2. **打开 DevTools**
   - 按 `F12` 或 `Ctrl+Shift+I` (Mac: `Cmd+Option+I`)
   - 或右键 → 检查

3. **打开命令面板**
   - 按 `Ctrl+Shift+P` (Mac: `Cmd+Shift+P`)
   - 输入 "screenshot" 或"capture"

4. **选择录制选项**
   - **Capture area screenshot** - 截取区域
   - **Capture full size screenshot** - 全页截图
   - **Capture screenshot** - 可视区域截图

> **注意**: Chrome DevTools 本身不直接支持视频录制，但可以截取高质量截图序列。

## 🎬 方法2: Puppeteer + FFmpeg (自动化，高质量)

### 安装依赖

```bash
npm install puppeteer
# 或全局安装
npm install -g puppeteer
```

### 创建录制脚本

创建 [`record-animation.js`](record-animation.js):

```javascript
const puppeteer = require('puppeteer');
const path = require('path');

(async () => {
    const browser = await puppeteer.launch({
        headless: false,
        args: [
            '--start-maximized',
            '--disable-infobars',
            '--window-size=1920,1080'
        ]
    });

    const page = await browser.newPage();
    await page.setViewport({ width: 1920, height: 1080 });

    // 导航到动画页面
    await page.goto('http://localhost:8000', {
        waitUntil: 'networkidle0'
    });

    console.log('开始录制...');

    // 开始屏幕录制 (需要 Chrome 95+)
    const client = await page.target().createCDPSession();
    
    // 启动录制
    await client.send('Page.startScreencast', {
        format: 'png',
        quality: 100,
        everyNthFrame: 1 // 捕获每一帧
    });

    // 等待动画完成 (30秒)
    await page.waitForTimeout(30000);

    console.log('录制完成！');
    
    await client.send('Page.stopScreencast');
    await browser.close();
})();
```

### 运行录制

```bash
node record-animation.js
```

## 🎞️ 方法3: OBS Studio (专业，免费)

### 安装 OBS Studio

```bash
# Ubuntu/Debian
sudo apt install obs-studio

# macOS
brew install --cask obs

# Windows
# 从 https://obsproject.com 下载安装
```

### 录制步骤

1. **启动 OBS Studio**

2. **添加浏览器源**
   - 点击 "Sources" → "+" → "Browser"
   - URL: `http://localhost:8000`
   - Width: 1920, Height: 1080
   - 勾选 "Shutdown source when not visible"
   - 勾选 "Refresh browser when scene becomes active"

3. **配置输出**
   - Settings → Output
   - Output Mode: Advanced
   - Encoder: x264 (CPU) 或 NVENC (GPU)
   - Rate Control: CBR
   - Bitrate: 6000-10000 Kbps

4. **开始录制**
   - 点击 "Start Recording"
   - 在浏览器中刷新页面开始动画
   - 等待 30 秒动画完成
   - 点击 "Stop Recording"

5. **输出位置**
   - 默认: `~/Videos/` (Linux/Mac)
   - 默认: `C:\Users\<你的用户名>\Videos\` (Windows)

## 🚀 方法4: Playwright (推荐，最强大)

### 安装 Playwright

```bash
npm install -D @playwright/test
npx playwright install chromium
```

### 创建录制脚本

创建 [`playwright-record.js`](playwright-record.js):

```javascript
const { chromium } = require('playwright');

(async () => {
    const browser = await chromium.launch({
        headless: false,
        args: ['--start-maximized']
    });

    const context = await browser.newContext({
        viewport: { width: 1920, height: 1080 },
        recordVideo: {
            dir: './videos/',
            size: { width: 1920, height: 1080 }
        }
    });

    const page = await context.newPage();
    
    console.log('打开动画页面...');
    await page.goto('http://localhost:8000');
    
    console.log('等待动画完成 (30秒)...');
    await page.waitForTimeout(30000);
    
    console.log('关闭浏览器，保存视频...');
    await context.close();
    await browser.close();
    
    console.log('✅ 视频已保存到 ./videos/ 目录');
})();
```

### 运行录制

```bash
node playwright-record.js
```

视频将自动保存到 `./videos/` 目录，格式为 `.webm`

### 转换为 MP4

```bash
# 安装 FFmpeg
sudo apt install ffmpeg  # Ubuntu/Debian
brew install ffmpeg      # macOS

# 转换视频
ffmpeg -i videos/video.webm -c:v libx264 -preset slow -crf 22 nu-animation.mp4
```

## 🎨 方法5: Chrome Headless + FFmpeg

### 使用无头 Chrome 录制

```bash
# 启动 Chrome headless 并保存截图序列
google-chrome --headless \
  --disable-gpu \
  --window-size=1920,1080 \
  --screenshot=frame_%04d.png \
  http://localhost:8000

# 使用 FFmpeg 将截图序列转换为视频
ffmpeg -framerate 60 \
  -pattern_type glob \
  -i 'frame_*.png' \
  -c:v libx264 \
  -pix_fmt yuv420p \
  -crf 23 \
  output.mp4
```

## 📊 方法对比

| 方法 | 难度 | 质量 | 自动化 | 推荐度 |
|------|------|------|--------|--------|
| Chrome DevTools | ⭐ | ⭐⭐ | ❌ | ⭐⭐ |
| Puppeteer | ⭐⭐⭐ | ⭐⭐⭐⭐ | ✅ | ⭐⭐⭐⭐ |
| OBS Studio | ⭐⭐ | ⭐⭐⭐⭐⭐ | ❌ | ⭐⭐⭐⭐⭐ |
| Playwright | ⭐⭐ | ⭐⭐⭐⭐⭐ | ✅ | ⭐⭐⭐⭐⭐ |
| Headless Chrome | ⭐⭐⭐⭐ | ⭐⭐⭐ | ✅ | ⭐⭐⭐ |

## 🎯 推荐方案

### 快速预览
使用 **OBS Studio** - 简单直观，适合快速录制

### 自动化生产
使用 **Playwright** - 可编程，可重复，适合 CI/CD

### 专业制作
使用 **OBS Studio** + 后期编辑软件（如 DaVinci Resolve）

## 🔧 高级选项

### 添加背景音乐

```bash
ffmpeg -i animation.mp4 \
  -i background-music.mp3 \
  -c:v copy \
  -c:a aac \
  -map 0:v:0 \
  -map 1:a:0 \
  -shortest \
  animation-with-music.mp4
```

### 调整帧率

```bash
# 降低到 30fps（减小文件大小）
ffmpeg -i animation.mp4 -r 30 animation-30fps.mp4

# 提高到 60fps（更流畅）
ffmpeg -i animation.mp4 -r 60 animation-60fps.mp4
```

### 压缩视频

```bash
# 高质量压缩
ffmpeg -i animation.mp4 \
  -c:v libx264 \
  -preset slow \
  -crf 22 \
  animation-compressed.mp4

# 极限压缩（适合网络分享）
ffmpeg -i animation.mp4 \
  -c:v libx264 \
  -preset veryslow \
  -crf 28 \
  -vf scale=1280:720 \
  animation-small.mp4
```

### 生成 GIF

```bash
# 转换为 GIF
ffmpeg -i animation.mp4 \
  -vf "fps=15,scale=800:-1:flags=lanczos" \
  -c:v gif \
  animation.gif

# 使用 gifsicle 优化
gifsicle -O3 --colors 256 animation.gif -o animation-optimized.gif
```

## 💡 录制技巧

1. **使用固定分辨率**: 1920x1080 是标准 Full HD
2. **设置高帧率**: 至少 30fps，推荐 60fps
3. **关闭浏览器扩展**: 避免干扰录制
4. **使用隐身模式**: 避免缓存影响
5. **预热动画**: 先运行一次确保加载完成
6. **录制多次**: 选择最佳效果
7. **后期编辑**: 剪辑、添加字幕、背景音乐

## 🐛 常见问题

### Q: 视频卡顿怎么办？
A: 降低录制分辨率或关闭其他应用释放资源

### Q: 文件太大怎么办？
A: 使用 FFmpeg 压缩或降低码率

### Q: 颜色不准确怎么办？
A: 检查浏览器颜色配置，使用 sRGB 色彩空间

### Q: Playwright 录制的视频是 WebM 格式？
A: 使用 FFmpeg 转换为 MP4：
```bash
ffmpeg -i video.webm -c:v libx264 -crf 23 video.mp4
```

## 📚 参考资源

- [Playwright 录制文档](https://playwright.dev/docs/videos)
- [OBS Studio 官网](https://obsproject.com)
- [FFmpeg 文档](https://ffmpeg.org/documentation.html)
- [Puppeteer API](https://pptr.dev)

---

选择最适合你的方法开始录制吧！🎬