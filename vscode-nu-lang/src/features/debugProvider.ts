import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs/promises';
import { BinaryManager } from '../services/binaryManager';
import { BuildService } from '../services/buildService';
import { SourcemapService } from '../services/sourcemapService';
import { DebugSyncController } from './debugSync';

export class NuDebugConfigurationProvider implements vscode.DebugConfigurationProvider {
    private debugSyncController: DebugSyncController;

    constructor(
        private binaryManager: BinaryManager,
        private buildService: BuildService,
        private sourcemapService: SourcemapService
    ) {
        this.debugSyncController = new DebugSyncController(sourcemapService);
    }

    /**
     * 当用户按 F5 且没有 launch.json 配置时调用
     */
    async provideDebugConfigurations(
        folder: vscode.WorkspaceFolder | undefined
    ): Promise<vscode.DebugConfiguration[]> {
        return [
            {
                type: 'nu-lang',
                request: 'launch',
                name: 'Debug Nu File',
                program: '${file}',
                args: [],
                cwd: '${workspaceFolder}'
            }
        ];
    }

    /**
     * 解析调试配置，在启动调试会话前调用
     */
    async resolveDebugConfiguration(
        folder: vscode.WorkspaceFolder | undefined,
        config: vscode.DebugConfiguration,
        token?: vscode.CancellationToken
    ): Promise<vscode.DebugConfiguration | null> {
        // 如果没有配置，创建默认配置
        if (!config.type && !config.request && !config.name) {
            const editor = vscode.window.activeTextEditor;
            
            if (!editor || !editor.document.fileName.endsWith('.nu')) {
                vscode.window.showErrorMessage('Please open a .nu file to debug');
                return null;
            }

            config = {
                type: 'nu-lang',
                request: 'launch',
                name: 'Debug Nu File',
                program: editor.document.fileName,
                args: [],
                cwd: path.dirname(editor.document.fileName)
            };
        }

        // 获取 Nu 文件路径
        let nuFilePath = config.program;
        
        // 处理 VSCode 变量
        if (nuFilePath.includes('${file}')) {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showErrorMessage('No active editor');
                return null;
            }
            nuFilePath = editor.document.fileName;
        }

        if (nuFilePath.includes('${workspaceFolder}')) {
            if (folder) {
                nuFilePath = nuFilePath.replace('${workspaceFolder}', folder.uri.fsPath);
            }
        }

        // 验证是 .nu 文件
        if (!nuFilePath.endsWith('.nu')) {
            vscode.window.showErrorMessage('Debug target must be a .nu file');
            return null;
        }

        // 保存文件（如果需要）
        const document = vscode.window.activeTextEditor?.document;
        if (document && document.isDirty && document.fileName === nuFilePath) {
            await document.save();
        }

        // 构建带调试信息的二进制
        const buildResult = await vscode.window.withProgress(
            {
                location: vscode.ProgressLocation.Notification,
                title: 'Building for debug...',
                cancellable: false
            },
            async () => {
                return await this.buildService.buildForDebug(nuFilePath);
            }
        );

        if (!buildResult.success || !buildResult.binaryPath) {
            vscode.window.showErrorMessage(
                `Build failed: ${buildResult.error || 'Unknown error'}`,
                'Show Details'
            ).then(selection => {
                if (selection === 'Show Details' && buildResult.stderr) {
                    const channel = vscode.window.createOutputChannel('Nu Debug Build');
                    channel.appendLine(buildResult.stderr);
                    channel.show();
                }
            });
            return null;
        }

        // 检测可用的调试器
        const debuggerType = await this.detectDebugger();
        
        if (!debuggerType) {
            const choice = await vscode.window.showErrorMessage(
                'No debugger extension found. Please install CodeLLDB or C/C++ extension.',
                'Install CodeLLDB',
                'Install C/C++'
            );
            
            if (choice === 'Install CodeLLDB') {
                vscode.commands.executeCommand('workbench.extensions.installExtension', 'vadimcn.vscode-lldb');
            } else if (choice === 'Install C/C++') {
                vscode.commands.executeCommand('workbench.extensions.installExtension', 'ms-vscode.cpptools');
            }
            
            return null;
        }

        // 获取对应的 Rust 文件路径
        const rsFilePath = nuFilePath.replace(/\.nu$/, '.rs');

        // 打开 Nu-Rust 双窗口并启动同步
        await this.debugSyncController.openNuFileForDebug(nuFilePath, rsFilePath);
        this.debugSyncController.startSync();

        // 监听调试会话结束，停止同步
        const sessionListener = vscode.debug.onDidTerminateDebugSession(() => {
            this.debugSyncController.stopSync();
            sessionListener.dispose();
        });

        // 生成调试配置
        const debugConfig = await this.createDebugConfiguration(
            debuggerType,
            nuFilePath,
            buildResult.binaryPath,
            config.args || [],
            config.cwd || path.dirname(nuFilePath)
        );

        return debugConfig;
    }

    /**
     * 检测可用的调试器
     */
    private async detectDebugger(): Promise<'lldb' | 'cppdbg' | 'cppvsdbg' | null> {
        // 检查是否安装了 CodeLLDB
        const lldbExtension = vscode.extensions.getExtension('vadimcn.vscode-lldb');
        if (lldbExtension) {
            return 'lldb';
        }

        // 检查是否安装了 C/C++ 扩展
        const cppExtension = vscode.extensions.getExtension('ms-vscode.cpptools');
        if (cppExtension) {
            // 根据平台选择调试器
            if (process.platform === 'win32') {
                return 'cppvsdbg'; // Windows 上优先使用 MSVC 调试器
            } else if (process.platform === 'darwin') {
                return 'lldb'; // macOS 上使用 LLDB
            } else {
                return 'cppdbg'; // Linux 上使用 GDB
            }
        }

        return null;
    }

    /**
     * 创建调试配置
     */
    private async createDebugConfiguration(
        debuggerType: 'lldb' | 'cppdbg' | 'cppvsdbg',
        nuFilePath: string,
        binaryPath: string,
        args: string[],
        cwd: string
    ): Promise<vscode.DebugConfiguration> {
        // 注意：调试时我们直接调试 Rust 代码（.rs），而不是 Nu 代码
        // 因为调试器需要实际的 Rust 源码来设置断点和单步执行
        
        const rsFilePath = nuFilePath.replace(/\.nu$/, '.rs');
        
        if (debuggerType === 'lldb') {
            // CodeLLDB 配置
            return {
                type: 'lldb',
                request: 'launch',
                name: 'Debug Nu File (LLDB)',
                program: binaryPath,
                args: args,
                cwd: cwd,
                stopOnEntry: false,  // 不在入口停止
                sourceLanguages: ['rust'],
                terminal: 'integrated',
                // 在 main 函数设置断点
                sourceMap: {
                    [rsFilePath]: nuFilePath
                }
            };
        } else if (debuggerType === 'cppdbg') {
            // C/C++ GDB/LLDB 配置
            return {
                type: 'cppdbg',
                request: 'launch',
                name: 'Debug Nu File (GDB)',
                program: binaryPath,
                args: args,
                cwd: cwd,
                stopAtEntry: false,  // 不在入口停止
                MIMode: process.platform === 'darwin' ? 'lldb' : 'gdb',
                setupCommands: [
                    {
                        description: 'Enable pretty-printing for gdb',
                        text: '-enable-pretty-printing',
                        ignoreFailures: true
                    },
                    {
                        description: 'Set breakpoint at main',
                        text: `-break-insert ${path.basename(rsFilePath)}:main`,
                        ignoreFailures: false
                    }
                ],
                sourceFileMap: {
                    [rsFilePath]: nuFilePath
                }
            };
        } else {
            // Visual Studio Windows 调试器配置
            return {
                type: 'cppvsdbg',
                request: 'launch',
                name: 'Debug Nu File (MSVC)',
                program: binaryPath,
                args: args,
                cwd: cwd,
                stopAtEntry: false,  // 不在入口停止
                sourceFileMap: {
                    [rsFilePath]: nuFilePath
                }
            };
        }
    }
}