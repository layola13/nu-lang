# nubp (nu-blueprint) 设计规范 v1.0

**版本:** 1.1.0  
**日期:** 2025-12-25  
**状态:** 设计草案 (Enhanced)  
**目标:** UE5 蓝图的高密度文本表示  
**设计原则:** 严格参考 nu-lang v1.6.3

---

## 1. 核心设计理念

### 1.1 设计目标

**nubp** 是 UE5 蓝图系统的高密度文本方言，旨在：

- **压缩视觉复杂度**: 将 100+ 节点的蓝图压缩为精简文本（目标 60-70% token 节省）
- **增强 AI 交互**: 让 LLM 能直接解析/生成/优化蓝图逻辑
- **支持版本控制**: 文本格式天然支持 git diff/merge
- **保持可读性**: 不像 code-golf，保持人类可读

### 1.2 核心压缩策略（参考 nu-lang）

```
✅ 压缩层级：
  - 蓝图事件/函数定义关键字 (Event → E)
  - 高频节点操作 (Get → G, Set → S)
  - 控制流结构 (Branch → ?, Loop → L)
  
❌ 保持不变：
  - 复杂蓝图节点全名 (AddMovementInput, SpawnActor)
  - 引脚连接符号 (::<PinName>)
  - 宏式构造函数 (Make Vector, Construct Widget)
  - UE5 特定类型 (FVector, FRotator)
```

---

## 2. 词法与可见性

### 2.1 可见性规则（沿用 nu-lang 逻辑）

**事件/函数**: 由**关键字大小写**决定
- **`E`** → Public Event (BlueprintCallable)
- **`e`** → Custom Event (Private)
- **`F`** → Public Function (BlueprintCallable)
- **`f`** → Private Function

**变量**: 由**标识符首字母**决定（Go 风格）
- `V Health` → Public Variable (BlueprintReadWrite)
- `V _health` → Private Variable (下划线前缀)

### 2.2 关键字映射表

| 类别 | nubp 关键字 | UE5 蓝图节点 | nu-lang 对应 | 备注 |
|------|-------------|--------------|--------------|------|
| **事件定义** | **E** | Event (Public) | `F` | 大写=Public |
| | **e** | Custom Event | `f` | 小写=Private |
| **函数定义** | **F** | Function (Public) | `F` | 同 nu |
| | **f** | Function (Private) | `f` | 同 nu |
| **变量声明** | **V** | Variable (Public) | - | 新增 |
| | **v** | Variable (Private/Mut) | `v` (let mut) | 沿用 nu |
| | **l** | Local Temp Pin | `l` (let) | 沿用 nu |
| **控制流** | **?** | Branch (If) | `?` | 沿用 nu |
| | **M** | Switch / Select | `M` (match) | 沿用 nu |
| | **L** | Loop / ForEach | `L` | 沿用 nu |
| | **<** | Return Node | `<` | 沿用 nu |
| **原子操作** | **G** | Get Variable | - | 高频节点 |
| | **S** | Set Variable | - | 高频节点 |
| | **P** | Print String | - | 调试专用 |
| | **DL** | Delay | - | 常用 |
| | **br** | Break | `br` | 沿用 nu |
| | **ct** | Continue | `ct` | 沿用 nu |
| **节点缩写** | **SP** | Spawn Actor | - | 高频 |
| | **D** | Destroy Actor | - | 高频 |
| | **C** | Construct / Make | - | 构造类 |
| **执行流** | **->** | Exec Pin 连接 | - | 默认线性 |
| **类型转换** | **a** | Cast To | `a` (as) | 沿用 nu |
| **引用/修饰** | **!** | Mutable Pin | `!` | 沿用 nu |
| | **&** | Reference Pin | `&` | 沿用 nu |

---

## 3. 类型系统

### 3.1 UE5 核心类型映射

| nubp 类型 | UE5 原类型 | nu-lang 对应 | 说明 |
|-----------|-----------|--------------|------|
| **V** | FVector | `V` (Vec) | 三维向量 |
| **R** | FRotator | `R` (Result) | 旋转器 |
| **T** | FTransform | - | 变换 |
| **Str** | FString/FText | `String` | 字符串 |
| **str** | FName | `str` | 名称（slice-like）|
| **A** | AActor | `A` (Arc) | Actor 引用 |
| **O\<T\>** | Optional\<T\> | `O` (Option) | 可选值 |
| **Arr\<T\>** | TArray\<T\> | `V` (Vec) | 数组 |
| **Map\<K,V\>** | TMap\<K,V\> | - | 映射 |
| **B** | Boolean | - | 布尔 |
| **f32/i32** | float/int32 | 同 nu | 数值类型 |

### 3.2 泛型与引脚（Turbofish-like）

**规则**: 引脚名称必须保留，使用 `::<PinName>` 语法（类似 nu-lang 的 Turbofish）

```nu
// 示例：SpawnActor 节点的 Class 引脚
SP::<Class>(BP_Enemy)::<Location>(V{0,0,100})

// 等价蓝图节点：
// SpawnActor
//   - Class Pin: BP_Enemy
//   - Location Pin: FVector(0,0,100)
```

---

## 4. 符号与控制流

### 4.1 核心操作符（继承 nu-lang）

| 符号 | 含义 | UE5 蓝图等价 | 语法规则 | nu-lang 来源 |
|------|------|--------------|----------|--------------|
| **<** | Return | Return Node | 语句首: `< Val` | ✅ 同 nu |
| **?** | Branch | Branch (If) | `? Cond { True: ..., False: ... }` | ✅ 同 nu |
| **M** | Switch | Switch on Enum/Int | `M Val { Case1: ..., _: ... }` | ✅ 同 nu (match) |
| **L** | Loop | For/While/ForEach | `L { ... }`, `L i: List { ... }` | ✅ 同 nu |
| **!** | Mutable | Mutable Pin (后缀) | `Health!` → Mutable Variable Pin | ✅ 同 nu |

### 4.2 内存/引用修饰（沿用 nu-lang）

| 符号 | 含义 | 规则 | nu-lang 来源 |
|------|------|------|--------------|
| **!** | Mutable Pin | **后缀**: `Health!` | ✅ 同 nu |
| **&** | Reference Pin | **前缀**: `&Actor` | ✅ 同 nu |
| ***** | Deref / Target | **前缀**: `*Self` | ✅ 同 nu |

### 4.3 执行流连接

```nu
// 线性执行（默认从上到下）
E BeginPlay {
    -> S Health = 100.0        // 第一个执行
    -> P "Game Started"        // 第二个执行
    -> SP::<Class>(BP_Enemy)   // 第三个执行
}

// 分支执行
E Tick {
    l h = G Health
    ? h <= 0 {
        True: -> D Actor       // 销毁
        False: -> P "Alive"    // 打印
    }
}
```

---

## 5. 蓝图节点映射规则

### 5.1 高频节点缩写（v1.1 扩展）

#### 5.1.1 基础操作

| nubp 语法 | UE5 蓝图节点 | 备注 |
|-----------|--------------|------|
| `G VarName` | Get Variable | 获取变量 |
| `S VarName = Val` | Set Variable | 设置变量 |
| `P "Msg"` | Print String | 打印字符串 |
| `DL 2.0` | Delay (2 seconds) | 延迟 |
| `SP::<Class>(BP_X)` | Spawn Actor | 生成 Actor |
| `D Actor` | Destroy Actor | 销毁 Actor |
| `a Target to BP_Player` | Cast To BP_Player | 类型转换 |

#### 5.1.2 方向向量（超高频）⭐

| nubp 缩写 | UE5 蓝图节点 | 说明 |
|-----------|--------------|------|
| **GVF** | GetActorForwardVector | 获取前向量 |
| **GVR** | GetActorRightVector | 获取右向量 |
| **GVU** | GetActorUpVector | 获取上向量 |
| **GVL** | GetActorLocation | 获取位置 |
| **GVRot** | GetActorRotation | 获取旋转 |
| **GVB** | GetActorBounds | 获取边界 |

**使用示例**:
```nu
l fwd = GVF      // GetActorForwardVector()
l right = GVR    // GetActorRightVector()
l pos = GVL      // GetActorLocation()
```

#### 5.1.3 输入系统（超高频）⭐

| nubp 缩写 | UE5 蓝图节点 | 说明 |
|-----------|--------------|------|
| **AXF** | Get Input Axis "Forward" | 前后输入 |
| **AXR** | Get Input Axis "Right" | 左右输入 |
| **AXU** | Get Input Axis "Up" | 上下输入 |
| **AXL** | Get Input Axis "LookUp" | 视角上下 |
| **AXY** | Get Input Axis "Turn" | 视角左右 |

**使用示例**:
```nu
l forward = AXF   // Get Input Axis "Forward"
l right = AXR     // Get Input Axis "Right"
l lookUp = AXL    // Get Input Axis "LookUp"
```

#### 5.1.4 移动与物理（高频）

| nubp 缩写 | UE5 蓝图节点 | 说明 |
|-----------|--------------|------|
| **AMI** | AddMovementInput | 添加移动输入 |
| **ACI** | AddControllerInput | 添加控制器输入 |
| **AF** | AddForce | 添加力 |
| **AI** | AddImpulse | 添加冲量 |
| **SAL** | SetActorLocation | 设置位置 |
| **SAR** | SetActorRotation | 设置旋转 |
| **SAT** | SetActorTransform | 设置变换 |

**使用示例**:
```nu
-> AMI(direction, 1.0)        // AddMovementInput(direction, 1.0)
-> SAL(V{0,0,100})            // SetActorLocation(FVector(0,0,100))
-> AF(V{0,0,500})             // AddForce(FVector(0,0,500))
```

#### 5.1.5 战斗系统（游戏专用）⭐

| nubp 缩写 | UE5 蓝图节点 | 说明 |
|-----------|--------------|------|
| **TKD** | TakeDamage | 受到伤害 |
| **APD** | ApplyDamage | 应用伤害 |
| **APDD** | ApplyPointDamage | 应用点伤害 |
| **APRD** | ApplyRadialDamage | 应用范围伤害 |
| **HEL** | Heal | 治疗 |
| **DIE** | Die / Destroy | 死亡/销毁 |

**使用示例**:
```nu
// 受到伤害并检查死亡
E OnHit(Damage: f32) {
    l h = G Health
    S Health = h - Damage
    ? h <= 0 { True: -> DIE }  // 死亡
}

// 应用范围伤害
-> APRD::<Origin>(GVL)::<Radius>(500.0)::<Damage>(50.0)
```

#### 5.1.6 UI/UMG（高频）⭐

| nubp 缩写 | UE5 蓝图节点 | 说明 |
|-----------|--------------|------|
| **CW** | Create Widget | 创建控件 |
| **ATV** | Add to Viewport | 添加到视口 |
| **RFP** | Remove from Parent | 从父级移除 |
| **SV** | Set Visibility | 设置可见性 |
| **ST** | Set Text | 设置文本 |
| **GPC** | Get Player Controller | 获取玩家控制器 |
| **SMM** | Set Input Mode UI Only | 仅UI输入模式 |
| **SMG** | Set Input Mode Game Only | 仅游戏输入模式 |

**使用示例**:
```nu
// 创建并显示主菜单
E ShowMenu {
    l widget = CW::<Class>(BP_MainMenu)
    -> ATV(widget)
    -> SMM  // 切换到UI输入
}

// 关闭UI
E CloseMenu {
    -> RFP
    -> SMG  // 切换回游戏输入
}
```

#### 5.1.7 AI/Behavior Tree（高频）

| nubp 缩写 | UE5 蓝图节点 | 说明 |
|-----------|--------------|------|
| **MT** | Move To | 移动到 |
| **MTA** | Move To Actor | 移动到Actor |
| **MTS** | Stop Movement | 停止移动 |
| **GBK** | Get Blackboard Key | 获取黑板键 |
| **SBK** | Set Blackboard Key | 设置黑板键 |
| **FP** | Find Path | 寻路 |
| **GAI** | Get AI Controller | 获取AI控制器 |

**使用示例**:
```nu
// AI 追击玩家
E ChasePlayer {
    l player = GBK::<Key>("TargetPlayer")
    -> MTA(player)::<AcceptanceRadius>(100.0)
}

// 巡逻逻辑
E Patrol {
    l waypoint = GBK::<Key>("NextWaypoint")
    -> MT(waypoint)
}
```

#### 5.1.8 音频/特效（常用）

| nubp 缩写 | UE5 蓝图节点 | 说明 |
|-----------|--------------|------|
| **PS** | Play Sound | 播放音效 |
| **PSL** | Play Sound at Location | 在位置播放音效 |
| **SPE** | Spawn Emitter | 生成粒子特效 |
| **SPEL** | Spawn Emitter at Location | 在位置生成特效 |
| **SPEA** | Spawn Emitter Attached | 生成附加特效 |

**使用示例**:
```nu
// 播放音效和特效
E OnFireWeapon {
    -> PS(SFX_Gunshot)
    -> SPEL::<Emitter>(FX_MuzzleFlash)::<Location>(GVL)
}
```

#### 5.1.9 摄像机/视角（常用）

| nubp 缩写 | UE5 蓝图节点 | 说明 |
|-----------|--------------|------|
| **GCC** | Get Control Rotation | 获取控制旋转 |
| **SCC** | Set Control Rotation | 设置控制旋转 |
| **GCV** | Get Camera Location | 获取摄像机位置 |
| **LKA** | Look At | 看向 |
| **ACP** | Add Controller Pitch | 添加俯仰 |
| **ACY** | Add Controller Yaw | 添加偏航 |

**使用示例**:
```nu
// 鼠标视角控制
E Tick {
    l mouseX = AXY  // Get Input Axis "Turn"
    l mouseY = AXL  // Get Input Axis "LookUp"
    -> ACY(mouseX)
    -> ACP(mouseY)
}
```

### 5.2 复杂节点（保持原名）

**规则**: 复杂节点保持完整名称，不压缩（类似 nu-lang 保留宏）

```nu
// ✅ 保持原名
AddMovementInput(Direction, ScaleValue)
SetActorLocation(NewLocation)
GetActorForwardVector()
ConstructWidget::<Class>(BP_Menu)

// ❌ 不要过度缩写
// AMovInput(...)  // 太晦涩
```

### 5.3 宏式构造（保持不变）

类似 nu-lang 的 `vec![]`、`println!()` 保持原样，nubp 的构造函数也不压缩：

```nu
// Make Vector (保持原名或用 V 简写)
V { X: 100, Y: 200, Z: 300 }  // 简写
Make Vector(X=100, Y=200, Z=300)  // 完整

// Make Rotator
R { Pitch: 0, Yaw: 90, Roll: 0 }

// Make Array
Arr[1, 2, 3]  // 简写
Make Array(Items: 1, 2, 3)  // 完整
```

---

## 6. 事件与函数定义

### 6.1 事件定义

```nu
// Public Event (BlueprintCallable)
E EventName {
    // 事件体
}

// Custom Event (Private)
e OnDamageReceived(Damage: f32) {
    l h = G Health
    S Health = h - Damage
}
```

### 6.2 函数定义

```nu
// Public Function
F CalculateDamage(Base: f32, Mult: f32) -> f32 {
    < Base * Mult  // return
}

// Private Function
f InternalCheck() -> B {
    l h = G Health
    < h > 0
}
```

### 6.3 变量定义

```nu
// Public Variable (BlueprintReadWrite)
#[Category("Stats")]
V Health: f32 = 100.0

// Private Variable
v _internalTimer: f32 = 0.0

// Local Temp (临时 Pin，函数内)
l tempValue = G Health * 2.0
```

---

## 7. 属性与元数据

### 7.1 蓝图属性（参考 nu-lang 的 #D）

| nubp 语法 | UE5 蓝图属性 | nu-lang 对应 |
|-----------|--------------|--------------|
| **#[EA]** | EditAnywhere | - |
| **#[BRW]** | BlueprintReadWrite | - |
| **#[BRO]** | BlueprintReadOnly | - |
| **#[BC]** | BlueprintCallable | - |
| **#[Cat("Name")]** | Category = "Name" | - |
| **#[...]** | 其他属性透传 | ✅ 同 nu |

**组合使用**:
```nu
#[EA, BRW, Cat("Stats")]
V MaxHealth: f32 = 100.0
```

**函数属性**:
```nu
#[BC, Cat("Combat")]
F TakeDamage(Amount: f32) {
    // ...
}
```

---

## 8. 完整示例：Third Person 角色移动

### 8.1 原版蓝图描述

```
Event BeginPlay
  → Set Health = 100.0

Event Tick
  → Get Input Axis "Forward" → ForwardValue
  → Get Input Axis "Right" → RightValue
  → Get Actor Forward Vector → FwdVec
  → Get Actor Right Vector → RightVec
  → Math: ForwardValue * FwdVec → DirX
  → Math: RightValue * RightVec → DirY
  → Math: DirX + DirY → Direction
  → Branch: Direction != Zero
      True: Add Movement Input (Direction, 1.0)
      False: (none)

Event InputAction Jump
  → Jump

Event InputAction JumpReleased
  → Stop Jumping
```

### 8.2 nubp 版本

```nu
// ============ 变量定义 ============
#[EA, BRW, Cat("Stats")]
V Health: f32 = 100.0

#[EA, BRW, Cat("Movement")]
V Speed: f32 = 600.0

// ============ 事件 ============
E BeginPlay {
    -> S Health = 100.0
}

E Tick {
    // v1.1 优化：使用超高频节点缩写
    l forward = AXF      // Get Input Axis "Forward"
    l right = AXR        // Get Input Axis "Right"
    
    // 计算移动方向（使用缩写）
    l fwdVec = GVF       // GetActorForwardVector()
    l rightVec = GVR     // GetActorRightVector()
    
    l direction = forward * fwdVec + right * rightVec  // 向量加法
    
    // 分支：如果有移动输入
    ? direction != V::ZERO {
        True: -> AMI(direction, 1.0)  // AddMovementInput
    }
}

E InputAction::<Action>(Jump) {
    -> Jump()
}

E InputAction::<Action>(JumpReleased) {
    -> StopJumping()
}

// ============ 自定义函数 ============
#[BC, Cat("Combat")]
F TakeDamage(Amount: f32) {
    l h = G Health
    S Health = h - Amount
    
    ? h <= 0 {
        True: -> D Actor  // 销毁自己
    }
}
```

### 8.3 压缩效果统计（v1.1 更新）

| 指标 | 原蓝图 | nubp v1.0 | nubp v1.1 | v1.1 节省率 |
|------|--------|-----------|-----------|-------------|
| **节点数** | ~25 个 | ~20 行 | ~15 行 | - |
| **Token 数**（估算）| ~1500 | ~650 | ~480 | **~68%** ⭐ |
| **视觉密度** | 需滚动浏览 | 一屏可见 | 半屏可见 | - |
| **核心改进** | - | 基础缩写 | 超高频节点缩写 | +11% |

**v1.1 关键优化**:
- `GetActorForwardVector()` → `GVF` (节省 21 字符)
- `Get Input Axis "Forward"` → `AXF` (节省 22 字符)
- `AddMovementInput` → `AMI` (节省 15 字符)
- 总压缩率从 57% 提升到 **68%**

---

## 9. 双向转换规则

### 9.1 Blueprint → nubp（导出规则）

| 蓝图元素 | 转换规则 | 示例 |
|----------|----------|------|
| Event Node | → `E` / `e` | `Event BeginPlay` → `E BeginPlay` |
| Function | → `F` / `f` | `Function GetHealth` → `F GetHealth()` |
| Get Variable | → `G` | `Get Health` → `G Health` |
| Set Variable | → `S` | `Set Health` → `S Health = 100` |
| Branch | → `?` | `Branch (Health > 0)` → `? Health > 0 { ... }` |
| ForEachLoop | → `L` | `ForEachLoop (Array)` → `L item: Array { ... }` |
| Print String | → `P` | `Print String ("Hi")` → `P "Hi"` |
| Return Node | → `<` | `Return (Value)` → `< Value` |

### 9.2 nubp → Blueprint（导入规则）

| nubp 语法 | 生成节点类型 | K2Node 类 |
|-----------|--------------|-----------|
| `E Name` | Event Node | `K2Node_Event` |
| `F Name()` | Function Entry | `K2Node_FunctionEntry` |
| `G Var` | Get Variable | `K2Node_VariableGet` |
| `S Var = Val` | Set Variable | `K2Node_VariableSet` |
| `? Cond { ... }` | Branch | `K2Node_IfThenElse` |
| `M Val { ... }` | Switch | `K2Node_Switch` |
| `L i: List { ... }` | ForEachLoop | `K2Node_ForEachLoop` |
| `< Val` | Return Node | `K2Node_Return` |

---

## 10. 不变元素（参考 nu-lang）

### 10.1 必须保留的部分

类似 nu-lang 保留宏和 Turbofish，nubp 必须保留：

1. **复杂蓝图节点全名**
   ```nu
   AddMovementInput(...)
   SpawnActorFromClass(...)
   LineTraceByChannel(...)
   ```

2. **引脚连接符（Turbofish-like）**
   ```nu
   SP::<Class>(BP_Enemy)::<Location>(V{0,0,100})
   ```

3. **宏式构造**
   ```nu
   Make Vector(X=1, Y=2, Z=3)
   Construct Widget(Class=BP_Menu)
   ```

4. **UE5 特定类型**
   ```nu
   FVector, FRotator, FTransform
   AActor, UActorComponent
   ```

### 10.2 可选压缩（逐步扩展）

| 阶段 | 支持范围 | 压缩率 |
|------|----------|--------|
| **Phase 1** | 核心节点（Event, Variable, Branch, Loop） | 45-55% |
| **Phase 1.1** | 超高频节点（方向向量、输入轴、移动）⭐ | **60-70%** |
| **Phase 2** | UMG Widget, Timeline, Animation | 65-72% |
| **Phase 3** | AI Behavior Tree, Blackboard | 68-75% |
| **Phase 4** | 完整蓝图系统 | 70-78% |

---

## 11. 语法规则总结

### 11.1 词法优先级（从高到低）

1. **引脚连接** `::<PinName>` （最高优先级，必保留）
2. **关键字** `E`, `F`, `l`, `v`, `?`, `M`, `L`
3. **节点缩写** `G`, `S`, `P`, `SP`, `D`
4. **符号** `<`, `!`, `&`, `*`, `->`
5. **标识符** 变量名、函数名、类型名

### 11.2 歧义消除规则

| 场景 | 规则 | 示例 |
|------|------|------|
| `<` 是 Return 还是小于？ | **语句首** = Return，其他 = 小于 | `< 5` vs `x < 5` |
| `!` 是 Try 还是 Mutable？ | **后缀** = Mutable，**前缀** = 逻辑非 | `Health!` vs `!isValid` |
| `V` 是 Variable 还是 Vector？ | **定义关键字** = Variable，**类型** = Vector | `V Health` vs `V{1,2,3}` |

---

## 12. AI 交互优化

### 12.1 LLM 友好性设计

| 特性 | 优化策略 | 效果 |
|------|----------|------|
| **Token 密度** | 单字母关键字 + 符号化 | 节省 55-65% |
| **上下文可读** | 保留复杂节点全名 | 易理解 |
| **结构化** | 缩进 + 分块 | 易解析 |
| **无歧义** | 明确语法规则 | 减少误判 |

### 12.2 喂给 AI 的最佳实践

```nu
// ✅ 推荐：保留上下文注释
// ============ Combat System ============
#[BC, Cat("Combat")]
F TakeDamage(Amount: f32) {
    l h = G Health
    S Health = h - Amount
    ? h <= 0 { True: -> D Actor }
}

// ❌ 避免：过度压缩失去可读性
F TD(A:f32){l h=G H;S H=h-A;?h<=0{T:->D A}}
```

---

## 13. 版本演进路线

### v1.0（当前）
- 核心语法定义
- Blueprint → nubp 单向导出
- 支持 Gameplay 蓝图（Event, Variable, Function, Flow）

### v1.1（未来）
- nubp → Blueprint 单向导入
- 支持 UMG Widget 蓝图
- 增加 Timeline、Curve 语法

### v1.2（未来）
- 部分双向转换（增量回填）
- 支持 Animation Blueprint
- 支持 AI Behavior Tree

### v2.0（长期）
- 完整双向转换（实验性）
- UE 编辑器插件集成
- 实时 Live Preview（文本 ↔ 节点图同步）

---

## 14. 与 nu-lang 的对应关系

| nu-lang 特性 | nubp 适配 | 说明 |
|--------------|-----------|------|
| `F` / `f` (函数可见性) | ✅ 完全继承 | Event / Function |
| `l` / `v` (let/let mut) | ✅ 完全继承 | Local / Mutable Var |
| `S` (struct) | ⚠️ 改为 Variable | UE5 无 struct 定义 |
| `<` (return) | ✅ 完全继承 | Return Node |
| `?` (if) | ✅ 完全继承 | Branch |
| `M` (match) | ✅ 完全继承 | Switch |
| `L` (loop) | ✅ 完全继承 | ForEach / While |
| `!` (mut 修饰) | ✅ 完全继承 | Mutable Pin |
| `&` / `*` | ✅ 完全继承 | Ref / Deref |
| `#D` (derive) | ⚠️ 改为属性 | 蓝图属性缩写 |
| Turbofish `::<T>` | ✅ 改为引脚 | Pin 连接符 |
| 宏保持原样 | ✅ 完全继承 | 复杂节点不压缩 |

**继承率**: ~85%  
**核心差异**: 蓝图是节点图，nu-lang 是代码，但设计哲学高度一致

---

## 15. 使用建议

### 15.1 何时使用 nubp

✅ **推荐场景**:
- 蓝图节点 > 100 个，视觉爆炸难管理
- 需要 AI 辅助生成/优化蓝图逻辑
- 多人协作，需要版本控制
- 需要快速文档化蓝图逻辑

❌ **不推荐场景**:
- 简单蓝图（< 50 节点），原生蓝图更直观
- 非程序员主导的项目（美术/策划为主）
- 已有完善的蓝图架构，迁移成本高

### 15.2 渐进式采用

```
Phase 1: 只读预览
  → 用 nubp 导出蓝图，喂给 AI 分析
  → 不修改原蓝图

Phase 2: 单向生成
  → 用 AI 生成 nubp 文本
  → 导入为蓝图节点

Phase 3: 混合开发
  → 部分蓝图用 nubp 文本编辑
  → 部分保持节点图

Phase 4: 文本优先（可选）
  → 主要写 nubp 文本
  → 节点图作为 Live Preview
```

---

## 总结

nubp (nu-blueprint) 严格遵循 nu-lang v1.6.3 的设计哲学：

- **高密度但可读**: 压缩定义关键字，保留语义核心
- **符号化控制流**: `<`, `?`, `M`, `L` 等单符号
- **大小写可见性**: Go 风格的 Public/Private
- **保留复杂部分**: 蓝图节点全名 = nu 的宏
- **Turbofish-like**: 引脚连接 `::<PinName>`

**预期效果**: 中等蓝图（100-300 节点）token 节省 **60-70%**（v1.1 超高频节点优化后），超越 nu-lang 在 Rust 上的压缩率。

**下一步**: 实现 Blueprint → nubp 的单向导出工具（UE5 Python Plugin）

---

**版权声明**: 本设计遵循 MIT License，与 nu-lang 项目保持一致。
