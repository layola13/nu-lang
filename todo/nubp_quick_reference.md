# nubp 节点缩写速查表 v1.1

## 基础操作

| 缩写 | 完整节点名 | 分类 |
|------|-----------|------|
| G | Get Variable | 变量 |
| S | Set Variable | 变量 |
| P | Print String | 调试 |
| DL | Delay | 流程 |
| SP | Spawn Actor | Actor |
| D | Destroy Actor | Actor |
| a | Cast To | 类型 |

## 方向向量 ⭐⭐⭐

| 缩写 | 完整节点名 | 说明 |
|------|-----------|------|
| GVF | GetActorForwardVector | 前向量 |
| GVR | GetActorRightVector | 右向量 |
| GVU | GetActorUpVector | 上向量 |
| GVL | GetActorLocation | 位置 |
| GVRot | GetActorRotation | 旋转 |
| GVB | GetActorBounds | 边界 |

## 输入系统 ⭐⭐⭐

| 缩写 | 完整节点名 | 说明 |
|------|-----------|------|
| AXF | Get Input Axis "Forward" | 前后 |
| AXR | Get Input Axis "Right" | 左右 |
| AXU | Get Input Axis "Up" | 上下 |
| AXL | Get Input Axis "LookUp" | 视角上下 |
| AXY | Get Input Axis "Turn" | 视角左右 |

## 移动与物理 ⭐⭐

| 缩写 | 完整节点名 | 说明 |
|------|-----------|------|
| AMI | AddMovementInput | 移动输入 |
| ACI | AddControllerInput | 控制器输入 |
| AF | AddForce | 添加力 |
| AI | AddImpulse | 添加冲量 |
| SAL | SetActorLocation | 设置位置 |
| SAR | SetActorRotation | 设置旋转 |
| SAT | SetActorTransform | 设置变换 |

## 战斗系统 ⭐⭐

| 缩写 | 完整节点名 | 说明 |
|------|-----------|------|
| TKD | TakeDamage | 受伤 |
| APD | ApplyDamage | 造成伤害 |
| APDD | ApplyPointDamage | 点伤害 |
| APRD | ApplyRadialDamage | 范围伤害 |
| HEL | Heal | 治疗 |
| DIE | Die / Destroy | 死亡 |

## UI/UMG ⭐⭐⭐

| 缩写 | 完整节点名 | 说明 |
|------|-----------|------|
| CW | Create Widget | 创建控件 |
| ATV | Add to Viewport | 添加到视口 |
| RFP | Remove from Parent | 移除 |
| SV | Set Visibility | 可见性 |
| ST | Set Text | 设置文本 |
| GPC | Get Player Controller | 玩家控制器 |
| SMM | Set Input Mode UI Only | UI模式 |
| SMG | Set Input Mode Game Only | 游戏模式 |

## AI/Behavior Tree ⭐

| 缩写 | 完整节点名 | 说明 |
|------|-----------|------|
| MT | Move To | 移动到 |
| MTA | Move To Actor | 移动到Actor |
| MTS | Stop Movement | 停止 |
| GBK | Get Blackboard Key | 黑板Get |
| SBK | Set Blackboard Key | 黑板Set |
| FP | Find Path | 寻路 |
| GAI | Get AI Controller | AI控制器 |

## 音频/特效

| 缩写 | 完整节点名 | 说明 |
|------|-----------|------|
| PS | Play Sound | 播放音效 |
| PSL | Play Sound at Location | 位置音效 |
| SPE | Spawn Emitter | 生成特效 |
| SPEL | Spawn Emitter at Location | 位置特效 |
| SPEA | Spawn Emitter Attached | 附加特效 |

## 摄像机/视角

| 缩写 | 完整节点名 | 说明 |
|------|-----------|------|
| GCC | Get Control Rotation | 控制旋转 |
| SCC | Set Control Rotation | 设置旋转 |
| GCV | Get Camera Location | 摄像机位置 |
| LKA | Look At | 看向 |
| ACP | Add Controller Pitch | 俯仰 |
| ACY | Add Controller Yaw | 偏航 |

---

## 使用提示

### 高频Top10（按使用频率）
1. **GVF/GVR** - 方向向量（几乎每个移动逻辑）
2. **AXF/AXR** - 输入轴（所有玩家控制）
3. **AMI** - 移动输入（角色移动）
4. **G/S** - 变量操作（无处不在）
5. **CW/ATV** - UI创建（菜单系统）
6. **GVL** - 位置获取（碰撞、生成）
7. **MTA/GBK** - AI系统（NPC行为）
8. **PS/SPE** - 音效特效（游戏反馈）
9. **APD** - 伤害系统（战斗）
10. **P** - 调试打印（开发必备）

### 记忆技巧
- **GV系列** = Get Vector（获取向量）
- **AX系列** = Axis（输入轴）
- **A系列** = Add（添加类操作）
- **S系列** = Set（设置类操作）
- **SP系列** = Spawn（生成类）
- **G系列** = Get（获取类）

---

**压缩效果**: 使用这些缩写后，典型蓝图的 token 可节省 **60-70%**
