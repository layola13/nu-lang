# nubp 示例：UMG 主菜单系统

## 场景说明

一个典型的游戏主菜单系统，包含：
- 主菜单显示/隐藏
- 按钮点击事件（开始游戏、设置、退出）
- 子菜单切换（设置菜单）
- 输入模式切换（UI ↔ Game）
- 动画过渡

---

## 原版蓝图描述（~150 节点）

```
=== BP_MainMenu Widget ===

Event Construct
  → Get Player Controller → PlayerController
  → Set Input Mode UI Only (PlayerController)
  → Show Mouse Cursor (PlayerController)
  → Play Animation (FadeIn)

Event OnStartButtonClicked
  → Play Sound (SFX_Click)
  → Play Animation (FadeOut)
  → Delay (0.5)
  → Remove from Parent
  → Set Input Mode Game Only (PlayerController)
  → Hide Mouse Cursor (PlayerController)
  → Open Level (GameMap)

Event OnSettingsButtonClicked
  → Play Sound (SFX_Click)
  → Create Widget (BP_SettingsMenu) → SettingsWidget
  → Add to Viewport (SettingsWidget)
  → Set Visibility (Hidden, self)

Event OnQuitButtonClicked
  → Play Sound (SFX_Click)
  → Quit Game

=== BP_SettingsMenu Widget ===

Event OnBackButtonClicked
  → Play Sound (SFX_Click)
  → Remove from Parent
  → Get Owning Player → PlayerController
  → Get All Widgets of Class (BP_MainMenu) → MainMenus
  → ForEach (MainMenus)
      → Set Visibility (Visible, item)

Event OnVolumeSliderChanged (NewValue: float)
  → Set Global Volume (NewValue)
  → Print String (concat("Volume: ", NewValue))
```

---

## nubp 版本（v1.1）

```nu
// ============ BP_MainMenu Widget ============
#[Cat("UI")]
V PlayerController: PlayerController = null

// 构造事件
E Construct {
    -> S PlayerController = GPC  // Get Player Controller
    -> SMM                        // Set Input Mode UI Only
    -> ShowMouseCursor(PlayerController)
    -> PlayAnimation(FadeIn)
}

// 开始游戏按钮
E OnStartButtonClicked {
    -> PS(SFX_Click)             // Play Sound
    -> PlayAnimation(FadeOut)
    -> DL 0.5                    // Delay
    -> RFP                       // Remove from Parent
    -> SMG                       // Set Input Mode Game Only
    -> HideMouseCursor(PlayerController)
    -> OpenLevel("GameMap")
}

// 设置按钮
E OnSettingsButtonClicked {
    -> PS(SFX_Click)
    l settingsWidget = CW::<Class>(BP_SettingsMenu)  // Create Widget
    -> ATV(settingsWidget)                           // Add to Viewport
    -> SV::<Target>(Self)::<Visibility>(Hidden)      // Set Visibility
}

// 退出按钮
E OnQuitButtonClicked {
    -> PS(SFX_Click)
    -> QuitGame
}


// ============ BP_SettingsMenu Widget ============

// 返回按钮
E OnBackButtonClicked {
    -> PS(SFX_Click)
    -> RFP  // Remove from Parent
    
    l controller = GPC
    l mainMenus = GetAllWidgetsOfClass::<Class>(BP_MainMenu)
    
    L menu: mainMenus {
        -> SV::<Target>(menu)::<Visibility>(Visible)
    }
}

// 音量滑块
E OnVolumeSliderChanged(NewValue: f32) {
    -> SetGlobalVolume(NewValue)
    -> P concat("Volume: ", NewValue)  // Print
}
```

---

## 压缩效果对比

| 指标 | 原蓝图 | nubp v1.1 | 节省率 |
|------|--------|-----------|--------|
| **节点数** | ~150 个 | ~35 行 | - |
| **Token 数**（估算）| ~3500 | ~1050 | **~70%** ⭐ |
| **文件大小** | ~25 KB | ~7 KB | ~72% |
| **可读性** | 需大量滚动 | 一屏可见 | - |

---

## 关键优化点

### v1.1 高频节点使用

| 原节点 | nubp 缩写 | 节省字符 |
|--------|----------|----------|
| `Create Widget` | `CW` | 11 |
| `Add to Viewport` | `ATV` | 13 |
| `Remove from Parent` | `RFP` | 15 |
| `Set Visibility` | `SV` | 11 |
| `Get Player Controller` | `GPC` | 18 |
| `Set Input Mode UI Only` | `SMM` | 19 |
| `Set Input Mode Game Only` | `SMG` | 23 |
| `Play Sound` | `PS` | 8 |
| `Delay` | `DL` | 3 |
| `Print String` | `P` | 10 |

**总节省**: ~131 字符 / 菜单

---

## AI 交互优化

### 喂给 Claude/GPT 的效果

**原蓝图**（需要截图 + 描述）:
```
# Context: ~3500 tokens
[截图] + "这是一个主菜单蓝图，有开始、设置、退出按钮..."
```

**nubp 版本**（直接文本）:
```
# Context: ~1050 tokens
E OnStartButtonClicked {
    -> PS(SFX_Click)
    -> PlayAnimation(FadeOut)
    ...
}
```

**AI 可直接**:
- ✅ 理解逻辑流程
- ✅ 提出优化建议（如缓存 PlayerController）
- ✅ 生成变体（如添加"继续游戏"按钮）
- ✅ 检测潜在 bug（如空指针检查）

---

## 最佳实践

### 1. 缓存常用引用
```nu
// ✅ 推荐：缓存 PlayerController
V PlayerController: PlayerController = null

E Construct {
    -> S PlayerController = GPC  // 只获取一次
}

// ❌ 避免：每次都获取
E OnButtonClick {
    l pc = GPC  // 重复操作
}
```

### 2. 使用注释分块
```nu
// ============ 主菜单逻辑 ============
E OnStartButtonClicked { ... }

// ============ 设置菜单逻辑 ============
E OnVolumeSliderChanged { ... }
```

### 3. 保留复杂节点全名
```nu
// ✅ 保持可读
-> GetAllWidgetsOfClass::<Class>(BP_MainMenu)

// ❌ 过度缩写
-> GAWOC::<C>(BP_MM)  // 难以理解
```

---

**总结**: UMG 系统是 nubp 最大收益的场景之一，token 节省可达 **70%**，尤其适合 AI 辅助开发。
