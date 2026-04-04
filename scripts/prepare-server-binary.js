const fs = require('fs')
const path = require('path')

const rootDir = path.resolve(__dirname, '..')
const serverDir = path.join(rootDir, 'server')
const targetDir = process.env.MCSHADER_SERVER_TARGET || 'release'
const fileName = process.platform === 'win32' ? 'vscode-mcshader.exe' : 'vscode-mcshader'
const platformKey = `${process.platform}-${process.arch}`

const sourcePath = path.join(serverDir, 'target', targetDir, fileName)
const destinationDir = path.join(serverDir, 'bin', platformKey)
const destinationPath = path.join(destinationDir, fileName)

if (!fs.existsSync(sourcePath)) {
    throw new Error(`Server binary not found: ${sourcePath}`)
}

fs.mkdirSync(destinationDir, { recursive: true })
fs.copyFileSync(sourcePath, destinationPath)

if (process.platform !== 'win32') {
    fs.chmodSync(destinationPath, 0o755)
}

console.log(`Prepared ${platformKey} server binary at ${destinationPath}`)
