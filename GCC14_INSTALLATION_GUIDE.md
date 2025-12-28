# GCC 14.1.0 安装指南

## 当前状态

✅ 已完成的步骤:
1. ✅ 安装构建基础工具 (build-essential)
2. ✅ 安装 GCC 依赖库 (libmpfr-dev, libgmp3-dev, libmpc-dev)
3. ✅ 下载 GCC 14.1.0 源码包
4. ✅ 解压源码包
5. ✅ 配置 GCC 编译选项

## 接下来需要做的

由于编译 GCC 需要很长时间(通常 30 分钟到几小时),我已经为你创建了一个自动化脚本来完成剩余的所有步骤。

### 方案 1: 使用自动化脚本 (推荐)

在终端中运行以下命令:

```bash
cd /home/sonygod/projects/nu
./install_gcc14.sh
```

这个脚本会自动完成:
- ⏱️  编译 GCC (这是最耗时的步骤)
- 📦 安装编译好的 GCC 到 `/usr/local/gcc-14.1.0`
- 🔧 配置 update-alternatives 使其成为默认编译器
- ✅ 验证安装结果

### 方案 2: 手动执行每个步骤

如果你想手动控制每一步,可以按顺序执行:

#### 步骤 1: 编译 GCC
```bash
cd /home/sonygod/projects/nu/gcc-14.1.0
make -j$(nproc)
```
⏱️ **预计时间**: 30分钟 - 3小时 (取决于 CPU 性能)

#### 步骤 2: 安装 GCC
```bash
sudo make install
```

#### 步骤 3: 配置为默认编译器
```bash
sudo update-alternatives --install /usr/bin/g++ g++ /usr/local/gcc-14.1.0/bin/g++-14.1.0 14
sudo update-alternatives --install /usr/bin/gcc gcc /usr/local/gcc-14.1.0/bin/gcc-14.1.0 14
```

#### 步骤 4: 验证安装
```bash
g++ --version
gcc --version
```

应该看到类似输出:
```
g++ (GCC) 14.1.0
gcc (GCC) 14.1.0
```

## 注意事项

1. **编译时间**: 编译过程会占用大量 CPU 资源,建议在不需要使用电脑时进行
2. **磁盘空间**: 确保有足够的磁盘空间(至少 10GB)
3. **内存**: 编译过程会使用较多内存,如果系统内存不足可能会失败
4. **权限**: 安装步骤需要 sudo 权限

## 后续使用

安装完成后,你就可以使用 C++20/23 的新特性了,比如:
- `std::format` (C++20)
- `std::chrono` 的增强功能
- 其他 C++20/23 特性

编译时可以使用:
```bash
g++ -std=c++20 your_file.cpp -o your_program
# 或
g++ -std=c++23 your_file.cpp -o your_program
```

## 故障排除

如果编译失败,常见原因:
1. **内存不足**: 尝试降低并行编译数 `make -j2` 而不是 `make -j$(nproc)`
2. **磁盘空间不足**: 清理一些空间后重试
3. **依赖库问题**: 重新检查依赖库是否都已安装

## 卸载方法 (如果需要)

如果将来需要卸载:
```bash
# 删除 update-alternatives 配置
sudo update-alternatives --remove g++ /usr/local/gcc-14.1.0/bin/g++-14.1.0
sudo update-alternatives --remove gcc /usr/local/gcc-14.1.0/bin/gcc-14.1.0

# 删除安装目录
sudo rm -rf /usr/local/gcc-14.1.0

# 删除源码目录 (可选)
rm -rf /home/sonygod/projects/nu/gcc-14.1.0
rm /home/sonygod/projects/nu/gcc-14.1.0.tar.gz