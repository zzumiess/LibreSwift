/* -*- Mode: C++; tab-width: 8; indent-tabs-mode: nil; c-basic-offset: 2 -*- */
/* vim: set ts=8 sts=2 et sw=2 tw=80: */
/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#include "ScriptLoadRequest.h"
#include "GeckoProfiler.h"

#include "mozilla/dom/Document.h"
#include "mozilla/dom/ScriptLoadContext.h"
#include "mozilla/dom/WorkerLoadContext.h"
#include "mozilla/dom/ScriptSettings.h"
#include "mozilla/HoldDropJSObjects.h"
#include "mozilla/StaticPrefs_dom.h"
#include "mozilla/Unused.h"
#include "mozilla/Utf8.h"  // mozilla::Utf8Unit

#include "js/SourceText.h"

#include "ModuleLoadRequest.h"
#include "nsContentUtils.h"
#include "nsICacheInfoChannel.h"
#include "nsIClassOfService.h"
#include "nsISupportsPriority.h"

using JS::SourceText;

namespace JS::loader {

//////////////////////////////////////////////////////////////
// ScriptFetchOptions
//////////////////////////////////////////////////////////////

NS_IMPL_CYCLE_COLLECTION(ScriptFetchOptions, mTriggeringPrincipal)

ScriptFetchOptions::ScriptFetchOptions(
    mozilla::CORSMode aCORSMode, const nsAString& aNonce,
    mozilla::dom::RequestPriority aFetchPriority,
    const ParserMetadata aParserMetadata, nsIPrincipal* aTriggeringPrincipal)
    : mCORSMode(aCORSMode),
      mNonce(aNonce),
      mFetchPriority(aFetchPriority),
      mParserMetadata(aParserMetadata),
      mTriggeringPrincipal(aTriggeringPrincipal) {}

ScriptFetchOptions::~ScriptFetchOptions() = default;

//////////////////////////////////////////////////////////////
// ScriptLoadRequest
//////////////////////////////////////////////////////////////

NS_INTERFACE_MAP_BEGIN_CYCLE_COLLECTION(ScriptLoadRequest)
  NS_INTERFACE_MAP_ENTRY(nsISupports)
NS_INTERFACE_MAP_END

NS_IMPL_CYCLE_COLLECTING_ADDREF(ScriptLoadRequest)
NS_IMPL_CYCLE_COLLECTING_RELEASE(ScriptLoadRequest)

NS_IMPL_CYCLE_COLLECTION_CLASS(ScriptLoadRequest)

NS_IMPL_CYCLE_COLLECTION_UNLINK_BEGIN(ScriptLoadRequest)
  NS_IMPL_CYCLE_COLLECTION_UNLINK(mFetchOptions, mOriginPrincipal, mBaseURL,
                                  mLoadedScript, mCacheInfo, mLoadContext)
  tmp->mScriptForBytecodeEncoding = nullptr;
  tmp->DropBytecodeCacheReferences();
NS_IMPL_CYCLE_COLLECTION_UNLINK_END

NS_IMPL_CYCLE_COLLECTION_TRAVERSE_BEGIN(ScriptLoadRequest)
  NS_IMPL_CYCLE_COLLECTION_TRAVERSE(mFetchOptions, mCacheInfo, mLoadContext,
                                    mLoadedScript)
NS_IMPL_CYCLE_COLLECTION_TRAVERSE_END

NS_IMPL_CYCLE_COLLECTION_TRACE_BEGIN(ScriptLoadRequest)
  NS_IMPL_CYCLE_COLLECTION_TRACE_JS_MEMBER_CALLBACK(mScriptForBytecodeEncoding)
NS_IMPL_CYCLE_COLLECTION_TRACE_END

ScriptLoadRequest::ScriptLoadRequest(
    ScriptKind aKind, nsIURI* aURI,
    mozilla::dom::ReferrerPolicy aReferrerPolicy,
    ScriptFetchOptions* aFetchOptions, const SRIMetadata& aIntegrity,
    nsIURI* aReferrer, LoadContextBase* aContext)
    : mKind(aKind),
      mState(State::CheckingCache),
      mFetchSourceOnly(false),
      mReferrerPolicy(aReferrerPolicy),
      mFetchOptions(aFetchOptions),
      mIntegrity(aIntegrity),
      mReferrer(aReferrer),
      mURI(aURI),
      mLoadContext(aContext),
      mEarlyHintPreloaderId(0) {
  MOZ_ASSERT(mFetchOptions);
  if (mLoadContext) {
    mLoadContext->SetRequest(this);
  }
}

ScriptLoadRequest::~ScriptLoadRequest() { DropJSObjects(this); }

void ScriptLoadRequest::SetReady() {
  MOZ_ASSERT(!IsFinished());
  mState = State::Ready;
}

void ScriptLoadRequest::Cancel() {
  mState = State::Canceled;
  if (HasScriptLoadContext()) {
    GetScriptLoadContext()->MaybeCancelOffThreadScript();
  }
}

void ScriptLoadRequest::DropBytecodeCacheReferences() {
  mCacheInfo = nullptr;
}

bool ScriptLoadRequest::HasScriptLoadContext() const {
  return HasLoadContext() && mLoadContext->IsWindowContext();
}

bool ScriptLoadRequest::HasWorkerLoadContext() const {
  return HasLoadContext() && mLoadContext->IsWorkerContext();
}

mozilla::dom::ScriptLoadContext* ScriptLoadRequest::GetScriptLoadContext() {
  MOZ_ASSERT(mLoadContext);
  return mLoadContext->AsWindowContext();
}

const mozilla::dom::ScriptLoadContext* ScriptLoadRequest::GetScriptLoadContext()
    const {
  MOZ_ASSERT(mLoadContext);
  return mLoadContext->AsWindowContext();
}

mozilla::loader::SyncLoadContext* ScriptLoadRequest::GetSyncLoadContext() {
  MOZ_ASSERT(mLoadContext);
  return mLoadContext->AsSyncContext();
}

mozilla::dom::WorkerLoadContext* ScriptLoadRequest::GetWorkerLoadContext() {
  MOZ_ASSERT(mLoadContext);
  return mLoadContext->AsWorkerContext();
}

mozilla::dom::WorkletLoadContext* ScriptLoadRequest::GetWorkletLoadContext() {
  MOZ_ASSERT(mLoadContext);
  return mLoadContext->AsWorkletContext();
}

ModuleLoadRequest* ScriptLoadRequest::AsModuleRequest() {
  MOZ_ASSERT(IsModuleRequest());
  return static_cast<ModuleLoadRequest*>(this);
}

const ModuleLoadRequest* ScriptLoadRequest::AsModuleRequest() const {
  MOZ_ASSERT(IsModuleRequest());
  return static_cast<const ModuleLoadRequest*>(this);
}

bool ScriptLoadRequest::IsCacheable() const {
  if (HasScriptLoadContext() && GetScriptLoadContext()->mIsInline) {
    return false;
  }

  return !mExpirationTime.IsExpired();
}

void ScriptLoadRequest::CacheEntryFound(LoadedScript* aLoadedScript) {
  MOZ_ASSERT(IsCheckingCache());
  MOZ_ASSERT(mURI);

  MOZ_ASSERT(mFetchOptions->IsCompatible(aLoadedScript->GetFetchOptions()));

  switch (mKind) {
    case ScriptKind::eClassic:
    case ScriptKind::eImportMap:
      MOZ_ASSERT(aLoadedScript->IsClassicScript());

      mLoadedScript = aLoadedScript;

      // Classic scripts can be set ready once the script itself is ready.
      mState = State::Ready;
      break;
    case ScriptKind::eModule:
      // NOTE: The cache entry has "module" kind, but it's not ModuleScript
      //       instance, given ModuleScript has GC pointers.
      MOZ_ASSERT(aLoadedScript->IsModuleScript());

      mLoadedScript = ModuleScript::FromCache(*aLoadedScript);

#ifdef DEBUG
      {
        bool equals = false;
        mURI->Equals(mLoadedScript->GetURI(), &equals);
        MOZ_ASSERT(equals);
      }
#endif

      mBaseURL = mLoadedScript->BaseURL();

      // Modules need to wait for fetching dependencies before setting to
      // Ready.  See also ModuleLoadRequest::DependenciesLoaded.
      mState = State::Fetching;
      break;
    case ScriptKind::eEvent:
      MOZ_ASSERT_UNREACHABLE("EventScripts are not using ScriptLoadRequest");
      break;
  }
}

void ScriptLoadRequest::NoCacheEntryFound() {
  MOZ_ASSERT(IsCheckingCache());
  MOZ_ASSERT(mURI);
  // At the time where we check in the cache, the mBaseURL is not set, as this
  // is resolved by the network. Thus we use the mURI, for checking the cache
  // and later replace the mBaseURL using what the Channel->GetURI will provide.
  switch (mKind) {
    case ScriptKind::eClassic:
    case ScriptKind::eImportMap:
      mLoadedScript = new ClassicScript(mReferrerPolicy, mFetchOptions, mURI);
      break;
    case ScriptKind::eModule:
      mLoadedScript = new ModuleScript(mReferrerPolicy, mFetchOptions, mURI);
      break;
    case ScriptKind::eEvent:
      MOZ_ASSERT_UNREACHABLE("EventScripts are not using ScriptLoadRequest");
      break;
  }
  mState = State::Fetching;
}

void ScriptLoadRequest::SetPendingFetchingError() {
  MOZ_ASSERT(IsCheckingCache());
  mState = State::PendingFetchingError;
}

void ScriptLoadRequest::MarkScriptForBytecodeEncoding(JSScript* aScript) {
  MOZ_ASSERT(!IsModuleRequest());
  MOZ_ASSERT(!mScriptForBytecodeEncoding);
  MarkForBytecodeEncoding();
  mScriptForBytecodeEncoding = aScript;
  HoldJSObjects(this);
}

static bool IsInternalURIScheme(nsIURI* uri) {
  return uri->SchemeIs("moz-extension") || uri->SchemeIs("resource") ||
         uri->SchemeIs("moz-src") || uri->SchemeIs("chrome");
}

void ScriptLoadRequest::SetBaseURLFromChannelAndOriginalURI(
    nsIChannel* aChannel, nsIURI* aOriginalURI) {
  // Fixup moz-extension: and resource: URIs, because the channel URI will
  // point to file:, which won't be allowed to load.
  if (aOriginalURI && IsInternalURIScheme(aOriginalURI)) {
    mBaseURL = aOriginalURI;
  } else {
    aChannel->GetURI(getter_AddRefs(mBaseURL));
  }
}

//////////////////////////////////////////////////////////////
// ScriptLoadRequestList
//////////////////////////////////////////////////////////////

ScriptLoadRequestList::~ScriptLoadRequestList() { CancelRequestsAndClear(); }

void ScriptLoadRequestList::CancelRequestsAndClear() {
  while (!isEmpty()) {
    RefPtr<ScriptLoadRequest> first = StealFirst();
    first->Cancel();
    // And just let it go out of scope and die.
  }
}

#ifdef DEBUG
bool ScriptLoadRequestList::Contains(ScriptLoadRequest* aElem) const {
  for (const ScriptLoadRequest* req = getFirst(); req; req = req->getNext()) {
    if (req == aElem) {
      return true;
    }
  }

  return false;
}
#endif  // DEBUG

}  // namespace JS::loader
