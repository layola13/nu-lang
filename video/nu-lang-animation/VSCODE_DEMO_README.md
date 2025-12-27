# VSCode 集成演示动画

展示 Nu Language 完整的 VSCode 开发体验。

## 🎬 演示内容

这个独立的演示页面展示了 Nu Language 在 VSCode 中的完整工作流程：

### 演示时间轴 (19秒)

| 时间 | 场景 | 内容 |
|------|------|------|
| 0-2s | VSCode 启动 | 窗口淡入，显示完整的 VSCode 界面 |
| 2-3s | 代码显示 | Nu 代码逐行显示，带语法高亮 |
| 3-4s | 设置断点 | 在第4行添加红色断点标记 |
| 4-6s | 右键菜单 | 显示上下文菜单，高亮"Compile Nu File" |
| 6-9s | 编译过程 | 显示编译进度 → 编译成功提示 |
| 9-12s | F5 提示 | 显示"Press F5 to Debug"动画提示 |
| 12-13s | 调试启动 | 调试工具栏出现，状态栏变为调试模式 |
| 13-15s | 断点命中 | 黄色高亮显示当前执行行 |
| 15-17s | 单步调试 | Step Over 按钮动画，执行移动到下一行 |
| 17-19s | 调试完成 | 调试结束，界面恢复正常 |

## 🎨 界面特性

### VSCode 窗口组件

✅ **标题栏**
- macOS/Windows 风格按钮
- 文件名显示
- VSCode Logo

✅ **活动栏** (左侧)
- 文件浏览器图标（激活状态）
- 搜索图标
- 设置图标

✅ **侧边栏**
- 文件树展示
- 当前文件高亮
- 悬停效果

✅ **编辑器**
- 行号显示
- Nu 语法高亮
- 断点标记
- 调试高亮行

✅ **状态栏**
- 语言模式显示
- 行列号
- 编译状态
- 调试状态指示

### 语法高亮颜色

```css
关键字 (F, f, l): #ff8800 (橙色)
函数名 (calculate, main): #dcdcaa (黄色)
类型 (i32): #4ec9b0 (青色)
数字 (2, 10): #b5cea8 (绿色)
字符串 ("Result: {}"): #ce9178 (橙棕色)
注释 (//): #6a9955 (绿色)
```

## 🚀 使用方法

### 1. 启动服务器

```bash
cd video/nu-lang-animation
python -m http.server 8000
```

### 2. 打开演示页面

访问: `http://localhost:8000/index-vscode-demo.html`

### 3. 键盘控制

| 按键 | 功能 |
|------|------|
| `Space` | 播放/暂停动画 |
| `R` | 重新播放 |
| `F5` | （预留）触发调试 |

## 📁 文件结构

```
video/nu-lang-animation/
├── index-vscode-demo.html      # VSCode 演示主页面
├── css/
│   ├── styles.css              # 通用样式
│   └── vscode-theme.css        # VSCode 主题样式
└── js/
    └── vscode-demo.js          # VSCode 演示动画脚本
```

## 🎯 演示的 VSCode 功能

### 1. 代码编辑
- ✅ Nu 语法高亮
- ✅ 行号显示
- ✅ 代码折叠（视觉效果）
- ✅ 悬停高亮

### 2. 编译功能
- ✅ 右键上下文菜单
- ✅ "Compile Nu File" 命令
- ✅ 编译进度提示
- ✅ 编译成功通知
- ✅ 状态栏状态更新

### 3. 调试功能
- ✅ 断点设置（第4行）
- ✅ F5 启动调试
- ✅ 调试工具栏
  - ▶️ Continue
  - ⏸️ Pause
  - ⏹️ Stop
  - ↷ Step Over
  - ↴ Step Into
  - ⏎ Step Out
- ✅ 当前执行行高亮
- ✅ 单步调试演示
- ✅ 调试状态栏

### 4. 状态指示
- ✅ 语言模式: "Nu Language"
- ✅ 编译状态: "✓ Compiled"
- ✅ 调试状态: "⏸️ Paused on breakpoint"
- ✅ 位置信息: "Ln 4, Col 5"

## 🎨 自定义

### 修改代码示例

编辑 [`index-vscode-demo.html`](index-vscode-demo.html) 第 85-94 行：

```html
<div class="code-line"><span class="token-comment">// Your comment</span></div>
<div class="code-line"><span class="token-keyword">F</span> <span class="token-function">your_function</span>(...) {</div>
...
```

### 修改颜色主题

编辑 [`css/vscode-theme.css`](css/vscode-theme.css)：

```css
/* 修改关键字颜色 */
.token-keyword {
    color: #your-color;
}

/* 修改断点颜色 */
.breakpoint-dot {
    background: #your-color;
}
```

### 修改动画时长

编辑 [`js/vscode-demo.js`](js/vscode-demo.js)：

```javascript
// 修改编译时间（默认 1.5秒）
tl.to('#compile-toast', {
    opacity: 1,
    duration: 2 // 改为 2秒
}, 7);
```

## 🔧 技术实现

### HTML 结构
- 使用 Flexbox 布局模拟 VSCode 界面
- 独立的组件层级（标题栏、活动栏、侧边栏、编辑器）
- 绝对定位的覆盖层（菜单、提示、调试高亮）

### CSS 样式
- VSCode Dark+ 主题配色
- 自定义滚动条样式
- Transition 和 Animation 动画
- 悬停交互效果

### JavaScript 动画
- GSAP 时间轴控制
- 精确的时间点触发
- 键盘事件监听
- 交互式控制

## 📊 与实际 VSCode 的对比

| 功能 | 演示版本 | 实际 VSCode |
|------|----------|-------------|
| 界面布局 | ✅ 高度还原 | 完全一致 |
| 语法高亮 | ✅ 手动实现 | LSP 驱动 |
| 右键菜单 | ✅ 模拟 | 完整功能 |
| 编译功能 | ✅ 动画演示 | 真实编译 |
| 断点功能 | ✅ 视觉效果 | 真实断点 |
| 调试功能 | ✅ 动画演示 | DAP 协议 |
| 变量查看 | ❌ 未实现 | ✅ 完整支持 |
| 代码补全 | ❌ 未实现 | ✅ IntelliSense |

## 🎥 录制建议

### 推荐设置
- **分辨率**: 1920x1080 (Full HD)
- **帧率**: 60fps
- **时长**: 19秒
- **格式**: MP4 (H.264)

### 录制步骤
1. 使用 OBS Studio 或 Playwright
2. 全屏录制浏览器窗口
3. 确保网络已加载完成（GSAP CDN）
4. 一次性录制完整动画
5. 使用 FFmpeg 后期处理

详见 [RECORDING_GUIDE.md](RECORDING_GUIDE.md)

## 🐛 已知限制

1. **静态代码**: 代码内容固定，不能真实编辑
2. **模拟交互**: 右键菜单和调试是预设动画
3. **无变量面板**: 未实现完整的调试变量查看
4. **单一文件**: 只展示一个文件，无多标签切换

## 🔮 未来增强

计划添加的功能：
- [ ] 真实的代码输入动画（打字效果）
- [ ] 变量查看面板
- [ ] Call Stack 显示
- [ ] 更多调试操作（Step Into, Step Out）
- [ ] 错误提示和诊断
- [ ] 代码补全动画
- [ ] Git 集成显示

## 📖 相关文档

- [主动画 README](README.md) - 完整的 Nu Language 介绍动画
- [录制指南](RECORDING_GUIDE.md) - 如何录制成视频
- [VSCode 扩展文档](../../vscode-nu-lang/README.md) - 真实的 VSCode 扩展

## 💡 使用建议

### 适用场景
✅ 演示 Nu Language 的 IDE 集成
✅ 展示开发工作流程
✅ 教学和培训材料
✅ 项目推广视频

### 组合使用
1. 主动画展示语言特性
2. VSCode 演示展示开发体验
3. 录制指南制作最终视频

---

🎬 开始体验 Nu Language 的专业开发环境！