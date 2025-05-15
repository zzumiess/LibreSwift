/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

// Note: this file is included in aboutDialog.xhtml and preferences/advanced.xhtml
// if MOZ_UPDATER is defined.

/* import-globals-from aboutDialog.js */

var { XPCOMUtils } = ChromeUtils.importESModule(
  "resource://gre/modules/XPCOMUtils.sys.mjs"
);

ChromeUtils.defineESModuleGetters(this, {
  AppUpdater: "resource://gre/modules/AppUpdater.sys.mjs",
  DownloadUtils: "resource://gre/modules/DownloadUtils.sys.mjs",
  UpdateUtils: "resource://gre/modules/UpdateUtils.sys.mjs",
});

XPCOMUtils.defineLazyServiceGetter(
  this,
  "AUS",
  "@mozilla.org/updates/update-service;1",
  "nsIApplicationUpdateService"
);

var UPDATING_MIN_DISPLAY_TIME_MS = 1500;

var gAppUpdater;

function onUnload(_aEvent) {
  if (gAppUpdater) {
    gAppUpdater.destroy();
    gAppUpdater = null;
  }
}

function appUpdater(options = {}) {
  this._appUpdater = new AppUpdater();
  
  this._appUpdateListener = (status, ...args) => {
    this._onAppUpdateStatus(status, ...args);
  };
  
  this._appUpdater.addListener(this._appUpdateListener);
  this.options = options;
  this.updatingMinDisplayTimerId = null;
  this.updateDeck = document.getElementById("updateDeck");

  this.bundle = Services.strings.createBundle(
    "chrome://browser/locale/browser.properties"
  );

  try {
    let manualURL = new URL(
      Services.urlFormatter.formatURLPref("app.update.url.manual")
    );

    for (const manualLink of document.querySelectorAll(".manualLink")) {
      let displayUrl = manualURL.origin + manualURL.pathname;
      manualLink.href = manualURL.href;
      document.l10n.setArgs(manualLink.closest("[data-l10n-id]"), { displayUrl });
    }

    document.getElementById("failedLink").href = manualURL.href;
  } catch (e) {
    console.error("Invalid manual update url.", e);
  }

  this._appUpdater.check();
}

appUpdater.prototype = {
  destroy() {
    this.stopCurrentCheck();
    if (this.updatingMinDisplayTimerId) {
      clearTimeout(this.updatingMinDisplayTimerId);
    }
  },

  stopCurrentCheck() {
    this._appUpdater.removeListener(this._appUpdateListener);
    this._appUpdater.stop();
  },

  get update() {
    return this._appUpdater.update;
  },

  get selectedPanel() {
    return this.updateDeck.selectedPanel;
  },

  _onAppUpdateStatus(status, ...args) {
    switch (status) {
      case AppUpdater.STATUS.UPDATE_DISABLED_BY_POLICY:
        this.selectPanel("policyDisabled");
        break;
      case AppUpdater.STATUS.READY_FOR_RESTART:
        this.selectPanel("apply");
        break;
      case AppUpdater.STATUS.OTHER_INSTANCE_HANDLING_UPDATES:
        this.selectPanel("otherInstanceHandlingUpdates");
        break;
      case AppUpdater.STATUS.DOWNLOADING: {
        const downloadStatus = document.getElementById("downloading");
        if (!args.length) {
          let maxSize = this.update.selectedPatch ? this.update.selectedPatch.size : -1;
          const transfer = DownloadUtils.getTransferTotal(0, maxSize);
          document.l10n.setArgs(downloadStatus, { transfer });
          this.selectPanel("downloading");
        } else {
          let [progress, max] = args;
          const transfer = DownloadUtils.getTransferTotal(progress, max);
          document.l10n.setArgs(downloadStatus, { transfer });
        }
        break;
      }
      case AppUpdater.STATUS.STAGING:
        this.selectPanel("applying");
        break;
      case AppUpdater.STATUS.CHECKING:
        this.checkingForUpdatesDelayPromise = new Promise(resolve => {
          this.updatingMinDisplayTimerId = setTimeout(resolve, UPDATING_MIN_DISPLAY_TIME_MS);
        });

        if (Services.policies.isAllowed("appUpdate")) {
          this.selectPanel("checkingForUpdates");
        } else {
          this.selectPanel("policyDisabled");
        }
        break;
      case AppUpdater.STATUS.CHECKING_FAILED:
        this.selectPanel("checkingFailed");
        break;
      case AppUpdater.STATUS.NO_UPDATES_FOUND:
        this.checkingForUpdatesDelayPromise.then(() =>
