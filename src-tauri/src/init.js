// Gemini App - Initialization Script
// 此腳本透過 Tauri 的 js_init_script 機制，在每次頁面載入時自動注入。
// 注意：此腳本會被包裝在 (function() { ... })() 中，
// 所以需要掛到全域的變數必須用 window.xxx 宣告。

// ========== 快捷鍵處理 ==========
document.addEventListener('keydown', async (e) => {
    // 監聽 Ctrl + N (或 Cmd + N)
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
        try {
            const settings = await window.__TAURI__.core.invoke('get_settings');
            if (!settings.enable_ctrl_n) return;

            e.preventDefault();
            e.stopPropagation();
            console.log('[Tauri] Ctrl+N detected: Simulating Ctrl+Shift+O for new chat');

            // 模擬 Ctrl + Shift + O
            const event = new KeyboardEvent('keydown', {
                key: 'o',
                code: 'KeyO',
                keyCode: 79,
                which: 79,
                ctrlKey: true,
                shiftKey: true,
                altKey: false,
                metaKey: false,
                bubbles: true,
                cancelable: true
            });

            document.dispatchEvent(event);

            const eventUp = new KeyboardEvent('keyup', {
                key: 'o',
                code: 'KeyO',
                keyCode: 79,
                which: 79,
                ctrlKey: true,
                shiftKey: true,
                bubbles: true,
                cancelable: true
            });
            document.dispatchEvent(eventUp);
        } catch (err) {
            console.error('[Tauri] Error fetching settings in shortcut handler:', err);
        }
    }
}, true); // 使用 Capture phase 確保優先攔截

// ========== 外部連結處理 ==========

// 輔助函式：透過 Tauri invoke 開啟 URL
async function openInBrowser(url) {
    try {
        console.log('[Tauri] Opening in system browser:', url);
        await window.__TAURI__.core.invoke('open_url', { url: url });
    } catch (err) {
        console.error('[Tauri] Failed to open link:', err);
    }
}

// 判斷 URL 是否應該在外部瀏覽器打開
function shouldOpenExternal(href, targetAttr) {
    if (!href || href.startsWith('javascript:')) return false;
    if (!href.startsWith('http://') && !href.startsWith('https://')) return false;
    try {
        const url = new URL(href);
        const currentHost = window.location.hostname;
        // 不同域名
        if (url.hostname !== currentHost && url.hostname !== '') return true;
        // target="_blank"
        if (targetAttr === '_blank') return true;
        // Gemini 分享連結
        if (url.pathname.startsWith('/share/') || url.searchParams.has('share')) return true;
    } catch (e) {}
    return false;
}

// 1. 覆寫 window.open — 攔截 target="_blank" 及 JS 觸發的新視窗
const originalOpen = window.open;
window.open = function(url, target, features) {
    if (url && (typeof url === 'string') && (url.startsWith('http://') || url.startsWith('https://'))) {
        openInBrowser(url);
        return null;
    }
    return originalOpen ? originalOpen.call(this, url, target, features) : null;
};

// 2. 攔截所有連結點擊 (Capture phase 最高優先)
document.addEventListener('click', function(e) {
    let target = e.target;
    while (target && target.tagName !== 'A') {
        target = target.parentElement;
    }
    if (!target || !target.href) return;

    const href = target.href;
    const targetAttr = target.getAttribute('target');

    if (shouldOpenExternal(href, targetAttr)) {
        e.preventDefault();
        e.stopPropagation();
        e.stopImmediatePropagation();
        openInBrowser(href);
    }
}, true);

// 3. 攔截滑鼠中鍵點擊 (auxclick)
document.addEventListener('auxclick', function(e) {
    let target = e.target;
    while (target && target.tagName !== 'A') {
        target = target.parentElement;
    }
    if (!target || !target.href) return;

    if (shouldOpenExternal(target.href, target.getAttribute('target'))) {
        e.preventDefault();
        e.stopPropagation();
        openInBrowser(target.href);
    }
}, true);

console.log('[Tauri] Init script loaded: shortcuts + link opener');
