/**
 * Wayland RDP Clipboard Bridge
 *
 * GNOME Shell extension that monitors clipboard changes and exposes them
 * via D-Bus for wayland-rdp-server and other applications.
 *
 * This extension exists because GNOME's Portal implementation does not emit
 * SelectionOwnerChanged signals, making it impossible for external applications
 * to detect when the user copies something on the Linux side.
 *
 * D-Bus Interface: org.wayland_rdp.Clipboard
 * Object Path: /org/wayland_rdp/Clipboard
 *
 * Signals:
 *   - ClipboardChanged(mime_types: as, content_hash: s)
 *   - PrimaryChanged(mime_types: as, content_hash: s)
 *
 * Methods:
 *   - GetText() -> s
 *   - GetMimeTypes() -> as
 *   - GetPrimaryText() -> s
 *   - Ping(msg: s) -> s
 *   - GetVersion() -> s
 *
 * Configuration via GSettings:
 *   gsettings set org.gnome.shell.extensions.wayland-rdp-clipboard poll-interval 500
 */

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import St from 'gi://St';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

// D-Bus interface definition
const DBUS_INTERFACE_XML = `
<node>
  <interface name="org.wayland_rdp.Clipboard">
    <!-- Signals -->
    <signal name="ClipboardChanged">
      <arg name="mime_types" type="as" direction="out"/>
      <arg name="content_hash" type="s" direction="out"/>
    </signal>

    <signal name="PrimaryChanged">
      <arg name="mime_types" type="as" direction="out"/>
      <arg name="content_hash" type="s" direction="out"/>
    </signal>

    <!-- Methods -->
    <method name="GetText">
      <arg name="text" type="s" direction="out"/>
    </method>

    <method name="GetMimeTypes">
      <arg name="types" type="as" direction="out"/>
    </method>

    <method name="GetPrimaryText">
      <arg name="text" type="s" direction="out"/>
    </method>

    <method name="Ping">
      <arg name="message" type="s" direction="in"/>
      <arg name="reply" type="s" direction="out"/>
    </method>

    <method name="GetVersion">
      <arg name="version" type="s" direction="out"/>
    </method>

    <method name="GetSettings">
      <arg name="settings" type="a{sv}" direction="out"/>
    </method>

    <!-- Properties -->
    <property name="PollInterval" type="u" access="read"/>
    <property name="IsMonitoring" type="b" access="read"/>
  </interface>
</node>
`;

const BUS_NAME = 'org.wayland_rdp.Clipboard';
const OBJECT_PATH = '/org/wayland_rdp/Clipboard';
const INTERFACE_NAME = 'org.wayland_rdp.Clipboard';
const VERSION = '1.0.0';

// Log levels
const LOG_NONE = 0;
const LOG_ERROR = 1;
const LOG_INFO = 2;
const LOG_DEBUG = 3;

/**
 * Simple hash function for content change detection
 * Uses djb2 algorithm - fast and sufficient for change detection
 */
function hashString(str) {
    if (!str) return '0';
    let hash = 5381;
    for (let i = 0; i < str.length; i++) {
        hash = ((hash << 5) + hash) + str.charCodeAt(i);
        hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash).toString(16);
}

/**
 * Logger with configurable levels
 */
class Logger {
    constructor() {
        this._level = LOG_INFO;
    }

    setLevel(levelStr) {
        switch (levelStr) {
            case 'none': this._level = LOG_NONE; break;
            case 'error': this._level = LOG_ERROR; break;
            case 'info': this._level = LOG_INFO; break;
            case 'debug': this._level = LOG_DEBUG; break;
            default: this._level = LOG_INFO;
        }
    }

    error(msg) {
        if (this._level >= LOG_ERROR)
            log(`[wayland-rdp-clipboard] ERROR: ${msg}`);
    }

    info(msg) {
        if (this._level >= LOG_INFO)
            log(`[wayland-rdp-clipboard] ${msg}`);
    }

    debug(msg) {
        if (this._level >= LOG_DEBUG)
            log(`[wayland-rdp-clipboard] DEBUG: ${msg}`);
    }
}

const logger = new Logger();

/**
 * D-Bus service implementation
 */
class ClipboardDBusService {
    constructor(settings) {
        this._settings = settings;
        this._dbusImpl = null;
        this._nameOwnerId = 0;
        this._isMonitoring = false;
    }

    export() {
        const nodeInfo = Gio.DBusNodeInfo.new_for_xml(DBUS_INTERFACE_XML);
        const interfaceInfo = nodeInfo.lookup_interface(INTERFACE_NAME);

        this._dbusImpl = Gio.DBusExportedObject.wrapJSObject(interfaceInfo, this);
        this._dbusImpl.export(Gio.DBus.session, OBJECT_PATH);

        this._nameOwnerId = Gio.DBus.session.own_name(
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            this._onNameAcquired.bind(this),
            this._onNameLost.bind(this)
        );

        logger.info(`D-Bus service exported on ${BUS_NAME}`);
    }

    unexport() {
        if (this._nameOwnerId) {
            Gio.DBus.session.unown_name(this._nameOwnerId);
            this._nameOwnerId = 0;
        }

        if (this._dbusImpl) {
            this._dbusImpl.unexport();
            this._dbusImpl = null;
        }

        logger.info('D-Bus service unexported');
    }

    _onNameAcquired() {
        logger.info(`Acquired D-Bus name: ${BUS_NAME}`);
    }

    _onNameLost() {
        logger.error(`Lost D-Bus name: ${BUS_NAME}`);
    }

    // Signal emitters
    emitClipboardChanged(mimeTypes, contentHash) {
        if (this._dbusImpl) {
            this._dbusImpl.emit_signal(
                'ClipboardChanged',
                new GLib.Variant('(ass)', [mimeTypes, contentHash])
            );
            logger.debug(`Emitted ClipboardChanged signal (hash: ${contentHash})`);
        }
    }

    emitPrimaryChanged(mimeTypes, contentHash) {
        if (this._dbusImpl) {
            this._dbusImpl.emit_signal(
                'PrimaryChanged',
                new GLib.Variant('(ass)', [mimeTypes, contentHash])
            );
            logger.debug(`Emitted PrimaryChanged signal (hash: ${contentHash})`);
        }
    }

    // Method implementations (called via D-Bus)
    GetText() {
        return new Promise((resolve) => {
            const clipboard = St.Clipboard.get_default();
            clipboard.get_text(St.ClipboardType.CLIPBOARD, (clip, text) => {
                resolve(text || '');
            });
        });
    }

    GetMimeTypes() {
        // St.Clipboard doesn't expose MIME types directly
        // Return common types that we support
        return ['text/plain', 'text/plain;charset=utf-8', 'UTF8_STRING', 'STRING', 'TEXT'];
    }

    GetPrimaryText() {
        return new Promise((resolve) => {
            const clipboard = St.Clipboard.get_default();
            clipboard.get_text(St.ClipboardType.PRIMARY, (clip, text) => {
                resolve(text || '');
            });
        });
    }

    Ping(message) {
        return `pong: ${message}`;
    }

    GetVersion() {
        return VERSION;
    }

    GetSettings() {
        return {
            'poll-interval': new GLib.Variant('u', this._settings.get_uint('poll-interval')),
            'monitor-clipboard': new GLib.Variant('b', this._settings.get_boolean('monitor-clipboard')),
            'monitor-primary': new GLib.Variant('b', this._settings.get_boolean('monitor-primary')),
            'log-level': new GLib.Variant('s', this._settings.get_string('log-level')),
            'emit-on-empty': new GLib.Variant('b', this._settings.get_boolean('emit-on-empty')),
            'deduplicate-window': new GLib.Variant('u', this._settings.get_uint('deduplicate-window')),
        };
    }

    // Property accessors
    get PollInterval() {
        return this._settings.get_uint('poll-interval');
    }

    get IsMonitoring() {
        return this._isMonitoring;
    }

    set IsMonitoring(value) {
        this._isMonitoring = value;
    }
}

/**
 * Clipboard monitor - polls clipboard and emits D-Bus signals on changes
 */
class ClipboardMonitor {
    constructor(dbusService, settings) {
        this._dbus = dbusService;
        this._settings = settings;
        this._clipboard = St.Clipboard.get_default();

        this._lastClipboardHash = null;
        this._lastPrimaryHash = null;
        this._lastClipboardTime = 0;
        this._lastPrimaryTime = 0;

        this._pollSourceId = null;
        this._settingsChangedId = null;
    }

    start() {
        if (this._pollSourceId) {
            return; // Already running
        }

        this._dbus.IsMonitoring = true;

        // Watch for settings changes
        this._settingsChangedId = this._settings.connect('changed', (settings, key) => {
            this._onSettingsChanged(key);
        });

        // Start polling
        this._startPolling();

        logger.info(`Clipboard monitoring started`);
    }

    stop() {
        if (this._settingsChangedId) {
            this._settings.disconnect(this._settingsChangedId);
            this._settingsChangedId = null;
        }

        this._stopPolling();
        this._dbus.IsMonitoring = false;

        logger.info('Clipboard monitoring stopped');
    }

    _startPolling() {
        const interval = this._settings.get_uint('poll-interval');

        // Initial poll
        this._poll();

        // Start polling timer
        this._pollSourceId = GLib.timeout_add(
            GLib.PRIORITY_DEFAULT,
            interval,
            () => {
                this._poll();
                return GLib.SOURCE_CONTINUE;
            }
        );

        logger.debug(`Polling started with interval: ${interval}ms`);
    }

    _stopPolling() {
        if (this._pollSourceId) {
            GLib.source_remove(this._pollSourceId);
            this._pollSourceId = null;
        }
    }

    _onSettingsChanged(key) {
        logger.debug(`Settings changed: ${key}`);

        if (key === 'poll-interval') {
            // Restart polling with new interval
            this._stopPolling();
            this._startPolling();
        } else if (key === 'log-level') {
            logger.setLevel(this._settings.get_string('log-level'));
        }
    }

    _poll() {
        if (this._settings.get_boolean('monitor-clipboard')) {
            this._pollClipboard();
        }

        if (this._settings.get_boolean('monitor-primary')) {
            this._pollPrimary();
        }
    }

    _pollClipboard() {
        this._clipboard.get_text(St.ClipboardType.CLIPBOARD, (clip, text) => {
            const now = GLib.get_monotonic_time() / 1000; // Convert to ms
            const dedupeWindow = this._settings.get_uint('deduplicate-window');
            const emitOnEmpty = this._settings.get_boolean('emit-on-empty');

            // Skip if empty and we don't emit on empty
            if (text === null && !emitOnEmpty) {
                return;
            }

            const hash = hashString(text || '');

            // Skip if same content
            if (hash === this._lastClipboardHash) {
                return;
            }

            // Skip if within deduplication window
            if (now - this._lastClipboardTime < dedupeWindow) {
                logger.debug(`Skipping clipboard change (within dedupe window)`);
                return;
            }

            this._lastClipboardHash = hash;
            this._lastClipboardTime = now;

            const mimeTypes = this._detectMimeTypes(text);

            logger.info(`Clipboard changed (hash: ${hash}, length: ${text ? text.length : 0})`);
            this._dbus.emitClipboardChanged(mimeTypes, hash);
        });
    }

    _pollPrimary() {
        this._clipboard.get_text(St.ClipboardType.PRIMARY, (clip, text) => {
            const now = GLib.get_monotonic_time() / 1000;
            const dedupeWindow = this._settings.get_uint('deduplicate-window');
            const emitOnEmpty = this._settings.get_boolean('emit-on-empty');

            if (text === null && !emitOnEmpty) {
                return;
            }

            const hash = hashString(text || '');

            if (hash === this._lastPrimaryHash) {
                return;
            }

            if (now - this._lastPrimaryTime < dedupeWindow) {
                logger.debug(`Skipping primary change (within dedupe window)`);
                return;
            }

            this._lastPrimaryHash = hash;
            this._lastPrimaryTime = now;

            const mimeTypes = this._detectMimeTypes(text);

            logger.info(`Primary selection changed (hash: ${hash}, length: ${text ? text.length : 0})`);
            this._dbus.emitPrimaryChanged(mimeTypes, hash);
        });
    }

    _detectMimeTypes(text) {
        if (!text) {
            return ['text/plain'];
        }

        const types = ['text/plain', 'text/plain;charset=utf-8', 'UTF8_STRING', 'STRING'];

        // Detect if content might be HTML
        if (text.includes('<') && text.includes('>') &&
            (text.includes('<html') || text.includes('<div') ||
             text.includes('<p') || text.includes('<span'))) {
            types.push('text/html');
        }

        // Detect if content might be a URL
        if (text.match(/^https?:\/\/\S+$/)) {
            types.push('text/uri-list');
            types.push('x-special/gnome-copied-files');
        }

        // Detect if content might be a file path
        if (text.match(/^(\/[\w.-]+)+\/?$/) || text.match(/^file:\/\//)) {
            types.push('text/uri-list');
            types.push('x-special/gnome-copied-files');
        }

        return types;
    }
}

/**
 * Main extension class
 */
export default class WaylandRdpClipboardExtension extends Extension {
    constructor(metadata) {
        super(metadata);
        this._settings = null;
        this._dbusService = null;
        this._monitor = null;
    }

    enable() {
        logger.info('Extension enabling...');

        // Load settings
        this._settings = this.getSettings();
        logger.setLevel(this._settings.get_string('log-level'));

        // Create and export D-Bus service
        this._dbusService = new ClipboardDBusService(this._settings);
        this._dbusService.export();

        // Create and start clipboard monitor
        this._monitor = new ClipboardMonitor(this._dbusService, this._settings);
        this._monitor.start();

        logger.info('Extension enabled');
    }

    disable() {
        logger.info('Extension disabling...');

        // Stop monitoring
        if (this._monitor) {
            this._monitor.stop();
            this._monitor = null;
        }

        // Unexport D-Bus service
        if (this._dbusService) {
            this._dbusService.unexport();
            this._dbusService = null;
        }

        this._settings = null;

        logger.info('Extension disabled');
    }
}
