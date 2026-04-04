import * as vscode from 'vscode'
import * as lc from 'vscode-languageclient/node'
import * as commands from './commands'
import { log } from './log'
import * as path from 'path'
import * as fs from 'fs'
import * as notification from './notification'

type ServerBinaryInfo = {
    platformKey: string
    fileName: string
}

const getServerBinaryInfo = (): ServerBinaryInfo => {
    const { platform, arch } = process
    const extension = platform === 'win32' ? '.exe' : ''

    switch (platform) {
        case 'win32':
        case 'linux':
        case 'darwin':
            return {
                platformKey: `${platform}-${arch}`,
                fileName: `vscode-mcshader${extension}`
            }
        default:
            throw new Error(`Unsupported platform: ${platform}/${arch}`)
    }
}

export class Extension {
    private statusBarItem: vscode.StatusBarItem | null = null
    private extensionContext: vscode.ExtensionContext | null = null
    private languageClient: lc.LanguageClient

    readonly extensionID = 'GeForceLegend.vscode-mcshader'

    updateStatus = (icon: string, text: string) => {
        this.statusBarItem?.dispose()
        this.statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left)
        this.statusBarItem.text = icon + ' [mc-shader] ' + text
        this.statusBarItem.show()
    }

    clearStatus = () => {
        this.statusBarItem?.dispose()
    }

    onStatusChange = (params: {
        status: 'loading' | 'ready' | 'failed' | 'clear'
        message: string
        icon: string
    }) => {
        switch (params.status) {
            case 'loading':
            case 'ready':
            case 'failed':
                this.updateStatus(params.icon, params.message)
                break
            case 'clear':
                this.clearStatus()
                break
        }
    }

    get context(): vscode.ExtensionContext {
        return this.extensionContext
    }

    get client(): lc.LanguageClient {
        return this.languageClient
    }

    private resolveServerPath = () => {
        const { platformKey, fileName } = getServerBinaryInfo()
        const candidates: string[] = []

        if (process.env['MCSHADER_DEBUG']) {
            const debugTarget = process.env['MCSHADER_DEBUG_TARGET']
            if (debugTarget) {
                candidates.push(this.extensionContext.asAbsolutePath(path.join('server', 'target', debugTarget, 'debug', fileName)))
            }
            candidates.push(this.extensionContext.asAbsolutePath(path.join('server', 'target', 'debug', fileName)))
        } else {
            candidates.push(this.extensionContext.asAbsolutePath(path.join('server', 'bin', platformKey, fileName)))
            candidates.push(this.extensionContext.asAbsolutePath(path.join('server', fileName)))
            if (process.platform === 'win32') {
                candidates.push(this.extensionContext.asAbsolutePath(path.join('server', 'vscode-mcshader.exe')))
            }
        }

        const serverPath = candidates.find((candidate) => fs.existsSync(candidate))
        if (serverPath) {
            return serverPath
        }

        throw new Error(
            `Language server binary not found for ${platformKey}. ` +
            `Expected one of:\n${candidates.map((candidate) => `- ${candidate}`).join('\n')}\n` +
            'Build or copy the matching binary into server/bin/<platform>-<arch>/.'
        )
    }

    public activate = async (context: vscode.ExtensionContext) => {
        this.extensionContext = context

        log.info('starting language server...')

        const serverPath = this.resolveServerPath()

        const server: lc.Executable = {
            command: serverPath,
            options: { env: { 'RUST_BACKTRACE': '1', ...process.env } }
        }
        const serverOption = {
            run: server,
            debug: server
        }
        this.languageClient = new lc.LanguageClient(
            'mcshader',
            'Minecraft Shaders LSP - Server',
            serverOption,
            {
                diagnosticCollectionName: 'mcshader',
                documentSelector: [{ scheme: 'file', language: 'glsl' }],
                synchronize: {
                    configurationSection: 'mcshader',
                },
            }
        )
        log.info('running with binary at path:\n\t', serverPath)
        this.updateStatus('$(loading~spin)', 'Starting...')
        await this.languageClient.start().then(() => {
            this.extensionContext.subscriptions.push(...commands.commandList(this))
            this.extensionContext.subscriptions.push(this.languageClient.onNotification(notification.StatusUpdateNoticationMethod, this.onStatusChange))

            log.info('language server started!')
        }).catch((err) => {
            log.error('failed to start language server!')
            log.error(err)

            this.updateStatus('&(error)', 'Start failed')
        })
    }

    deactivate = async () => {
        await this.languageClient.stop()
        this.context.subscriptions?.forEach((disposable) => disposable.dispose())
    }
}

export const activate = async (context: vscode.ExtensionContext) => {
    try {
        new Extension().activate(context)
    } catch (e) {
        log.error(`failed to activate extension: ${e}`)
        throw (e)
    }
}
