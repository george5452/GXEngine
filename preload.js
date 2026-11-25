
const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("desktop", {
    save: project =>
        ipcRenderer.invoke("project:save", project),

    open: () =>
        ipcRenderer.invoke("project:open"),

    export: (format, project) =>
        ipcRenderer.invoke("game:export", format, project)
});
