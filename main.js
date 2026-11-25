
const { app, BrowserWindow, ipcMain, dialog } = require("electron");
const fs = require("fs/promises");
const path = require("path");
const os = require("os");
const AdmZip = require("adm-zip");

let win;
let exporting = false;

function createWindow() {
    win = new BrowserWindow({
        width: 1600,
        height: 950,
        minWidth: 1000,
        minHeight: 650,
        title: "GXEngine",
        backgroundColor: "#191a1e",
        icon: path.join(__dirname, "assets/icon.png"),
        autoHideMenuBar: true,
        webPreferences: {
            preload: path.join(__dirname, "preload.js"),
            contextIsolation: true,
            nodeIntegration: false,
            sandbox: true
        }
    });

    win.loadFile("index.html");
    win.maximize();
}

app.whenReady().then(createWindow);

app.on("window-all-closed", () => {
    if (process.platform !== "darwin") app.quit();
});

app.on("activate", () => {
    if (!BrowserWindow.getAllWindows().length) createWindow();
});

ipcMain.handle("project:save", async (_, project) => {
    const result = await dialog.showSaveDialog(win, {
        defaultPath: `${project.name || "Untitled"}.gxproject`,
        filters: [
            { name: "GXEngine Project", extensions: ["gxproject"] }
        ]
    });

    if (result.canceled) return null;

    await fs.writeFile(
        result.filePath,
        JSON.stringify(project, null, 2)
    );

    return result.filePath;
});

ipcMain.handle("project:open", async () => {
    const result = await dialog.showOpenDialog(win, {
        properties: ["openFile"],
        filters: [
            { name: "GXEngine Project", extensions: ["gxproject"] }
        ]
    });

    if (result.canceled) return null;

    return JSON.parse(
        await fs.readFile(result.filePaths[0], "utf8")
    );
});

async function gameHTML(project) {
    const runtime = await fs.readFile(
        path.join(__dirname, "runtime.bundle.js"),
        "utf8"
    );

    const data = JSON.stringify(project)
        .replace(/</g, "\\u003c");

    return `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>GXEngine Game</title>
<style>
*{box-sizing:border-box}
body{margin:0;background:#111;overflow:hidden}
#game{position:fixed;inset:0}
canvas{width:100%;height:100%;display:block}
#hud{position:fixed;top:20px;left:20px;color:white;
font:20px system-ui;pointer-events:none;white-space:pre-wrap}
</style>
</head>
<body>
<div id="game"><canvas id="canvas"></canvas></div>
<div id="hud"></div>

<script>
${runtime.replace(/<\/script/gi, "<\\/script")}
</script>

<script>
(async()=>{
    const project = ${data};
    document.title = project.name || "GXEngine Game";

    const runtime = await GX.Runtime.create(
        document.getElementById("canvas"),
        project,
        false
    );

    runtime.onMessage = message => {
        document.getElementById("hud").textContent = message;
    };

    runtime.play();
})().catch(error=>{
    document.getElementById("hud").textContent = error.message;
});
</script>
</body>
</html>`;
}

ipcMain.handle("game:export", async (_, format, project) => {

    if (exporting) throw new Error("Export already in progress.");

    if (!["html", "zip", "exe"].includes(format)) {
        throw new Error("Invalid export format.");
    }

    const name = String(project.name || "GXGame")
        .replace(/[^a-zA-Z0-9 _-]/g, "")
        .trim() || "GXGame";

    const result = await dialog.showSaveDialog(win, {
        defaultPath: `${name}.${format}`,
        filters: [
            { name: format.toUpperCase(), extensions: [format] }
        ]
    });

    if (result.canceled) return null;

    exporting = true;
    let temp;

    try {
        const html = await gameHTML(project);

        if (format === "html") {
            await fs.writeFile(result.filePath, html);
        }

        if (format === "zip") {
            const zip = new AdmZip();

            zip.addFile("index.html", Buffer.from(html));

            zip.addFile(
                "project.gxproject",
                Buffer.from(JSON.stringify(project, null, 2))
            );

            zip.addFile(
                "README.txt",
                Buffer.from(
                    "Created with GXEngine.\nOpen index.html to play."
                )
            );

            zip.writeZip(result.filePath);
        }

        if (format === "exe") {

            if (process.platform !== "win32") {
                throw new Error("Windows EXE export requires Windows.");
            }

            temp = await fs.mkdtemp(
                path.join(os.tmpdir(), "gxengine-export-")
            );

            await fs.writeFile(
                path.join(temp, "index.html"),
                html
            );

            await fs.writeFile(
                path.join(temp, "package.json"),
                JSON.stringify({
                    name: "gx-game",
                    version: "1.0.0",
                    description: name,
                    author: "GXEngine Creator",
                    main: "main.js"
                })
            );

            await fs.writeFile(
                path.join(temp, "main.js"),
                `
const {app, BrowserWindow} = require("electron");

app.whenReady().then(()=>{
    const window = new BrowserWindow({
        width:1280,
        height:720,
        title:${JSON.stringify(name)},
        autoHideMenuBar:true,
        webPreferences:{
            nodeIntegration:false,
            contextIsolation:true,
            sandbox:true
        }
    });

    window.loadFile("index.html");
});

app.on("window-all-closed",()=>app.quit());
`
            );

            const {
                build,
                Platform,
                Arch
            } = require("electron-builder");

            const artifacts = await build({
                projectDir: temp,
                targets: Platform.WINDOWS.createTarget(
                    ["portable"], Arch.x64
                ),
                config: {
                    appId: "com.gxengine.game",
                    productName: name,
                    electronVersion: process.versions.electron,
                    files: [
                        "main.js",
                        "index.html",
                        "package.json"
                    ],
                    directories: {
                        output: path.join(temp, "output")
                    },
                    win: {
                        signAndEditExecutable: false
                    }
                }
            });

            const exe = artifacts.find(
                file => file.endsWith(".exe")
            );

            if (!exe) {
                throw new Error("Could not generate executable.");
            }

            await fs.copyFile(exe, result.filePath);
        }

        return result.filePath;

    } finally {

        exporting = false;

        if (temp) {
            await fs.rm(temp, {
                recursive: true,
                force: true
            });
        }
    }
});
