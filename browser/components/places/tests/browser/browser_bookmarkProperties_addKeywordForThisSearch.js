"use strict";

const TEST_URL =
  "http://mochi.test:8888/browser/browser/components/places/tests/browser/keyword_form.html";

let contentAreaContextMenu = document.getElementById("contentAreaContextMenu");

add_task(async function add_keyword() {
  await BrowserTestUtils.withNewTab(
    {
      gBrowser,
      url: TEST_URL,
    },
    async function (browser) {
      // We must wait for the context menu code to build metadata.
      await openContextMenuForContentSelector(
        browser,
        '#form1 > input[name="search"]'
      );

      await withBookmarksDialog(
        false,
        function () {
          gContextMenu.addKeywordForSearchField();
          contentAreaContextMenu.hidePopup();
        },
        async function (dialogWin) {
          let acceptBtn = dialogWin.document
            .getElementById("bookmarkpropertiesdialog")
            .getButton("accept");
          Assert.ok(acceptBtn.disabled, "Accept button is disabled");

          let promiseKeywordNotification = PlacesTestUtils.waitForNotification(
            "bookmark-keyword-changed",
            events => events.some(event => event.keyword === "kw")
          );

          fillBookmarkTextField("editBMPanel_keywordField", "kw", dialogWin);

          Assert.ok(!acceptBtn.disabled, "Accept button is enabled");

          acceptBtn.click();
          await promiseKeywordNotification;

          // After the notification, the keywords cache will update asynchronously.
          info("Check the keyword entry has been created");
          let entry;
          await TestUtils.waitForCondition(async function () {
            entry = await PlacesUtils.keywords.fetch("kw");
            return !!entry;
          }, "Unable to find the expected keyword");
          Assert.equal(entry.keyword, "kw", "keyword is correct");
          Assert.equal(entry.url.href, TEST_URL, "URL is correct");
          Assert.equal(
            entry.postData,
            "accenti%3D%E0%E8%EC%F2%F9&search%3D%25s",
            "POST data is correct"
          );
          let bm = await PlacesUtils.bookmarks.fetch({ url: TEST_URL });
          Assert.equal(
            bm.parentGuid,
            await PlacesUIUtils.defaultParentGuid,
            "Should have created the keyword in the right folder."
          );

          info("Check the charset has been saved");
          let pageInfo = await PlacesUtils.history.fetch(TEST_URL, {
            includeAnnotations: true,
          });
          Assert.equal(
            pageInfo.annotations.get(PlacesUtils.CHARSET_ANNO),
            "windows-1252",
            "charset is correct"
          );

          // Now check getShortcutOrURI.
          let data = await UrlbarUtils.getShortcutOrURIAndPostData("kw test");
          Assert.equal(
            getPostDataString(data.postData),
            "accenti=\u00E0\u00E8\u00EC\u00F2\u00F9&search=test",
            "getShortcutOrURI POST data is correct"
          );
          Assert.equal(data.url, TEST_URL, "getShortcutOrURI URL is correct");
        },
        () => PlacesUtils.bookmarks.eraseEverything()
      );
    }
  );
});

add_task(async function reopen_same_field() {
  await PlacesUtils.keywords.insert({
    url: TEST_URL,
    keyword: "kw",
    postData: "accenti%3D%E0%E8%EC%F2%F9&search%3D%25s",
  });
  registerCleanupFunction(async function () {
    await PlacesUtils.keywords.remove("kw");
  });
  // Reopening on the same input field should show the existing keyword.
  await BrowserTestUtils.withNewTab(
    {
      gBrowser,
      url: TEST_URL,
    },
    async function (browser) {
      // We must wait for the context menu code to build metadata.
      await openContextMenuForContentSelector(
        browser,
        '#form1 > input[name="search"]'
      );

      await withBookmarksDialog(
        true,
        function () {
          gContextMenu.addKeywordForSearchField();
          contentAreaContextMenu.hidePopup();
        },
        async function (dialogWin) {
          let elt = dialogWin.document.getElementById(
            "editBMPanel_keywordField"
          );
          Assert.equal(elt.value, "kw", "Keyword should be the previous value");

          let acceptBtn = dialogWin.document
            .getElementById("bookmarkpropertiesdialog")
            .getButton("accept");
          ok(!acceptBtn.disabled, "Accept button is enabled");
        },
        () => PlacesUtils.bookmarks.eraseEverything()
      );
    }
  );
});

add_task(async function open_other_field() {
  await PlacesUtils.keywords.insert({
    url: TEST_URL,
    keyword: "kw2",
    postData: "search%3D%25s",
  });
  registerCleanupFunction(async function () {
    await PlacesUtils.keywords.remove("kw2");
  });
  // Reopening on another field of the same page that has different postData
  // should not show the existing keyword.
  await BrowserTestUtils.withNewTab(
    {
      gBrowser,
      url: TEST_URL,
    },
    async function (browser) {
      // We must wait for the context menu code to build metadata.
      await openContextMenuForContentSelector(
        browser,
        '#form2 > input[name="search"]'
      );

      await withBookmarksDialog(
        true,
        function () {
          gContextMenu.addKeywordForSearchField();
          contentAreaContextMenu.hidePopup();
        },
        function (dialogWin) {
          let acceptBtn = dialogWin.document
            .getElementById("bookmarkpropertiesdialog")
            .getButton("accept");
          ok(acceptBtn.disabled, "Accept button is disabled");

          let elt = dialogWin.document.getElementById(
            "editBMPanel_keywordField"
          );
          is(elt.value, "");
        },
        () => PlacesUtils.bookmarks.eraseEverything()
      );
    }
  );
});

function getPostDataString(stream) {
  let sis = Cc["@mozilla.org/scriptableinputstream;1"].createInstance(
    Ci.nsIScriptableInputStream
  );
  sis.init(stream);
  return sis.read(stream.available()).split("\n").pop();
}
