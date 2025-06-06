// |reftest| shell-option(--enable-explicit-resource-management) skip-if(!(this.hasOwnProperty('getBuildConfiguration')&&getBuildConfiguration('explicit-resource-management'))||!xulRuntime.shell) async -- explicit-resource-management is not enabled unconditionally, requires shell-options
// Copyright (C) 2024 the V8 project authors. All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.

/*---
description: Throws a suppressed error from errors in disposal.
includes: [asyncHelpers.js]
flags: [async]
features: [explicit-resource-management]
---*/

// A suppressed error from two errors in disposal -----------------------------
asyncTest(async function() {
  let firstDisposeError =
      new Test262Error('The first Symbol.asyncDispose is throwing!');
  let secondDisposeError =
      new Test262Error('The second Symbol.asyncDispose is throwing!');

  async function TestTwoDisposeMethodsThrow() {
    await using x = {
      value: 1,
      [Symbol.asyncDispose]() {
        throw firstDisposeError;
      }
    };
    await using y = {
      value: 1,
      [Symbol.asyncDispose]() {
        throw secondDisposeError;
      }
    };
  };

  await assert.throwsAsync(
      SuppressedError, () => TestTwoDisposeMethodsThrow(),
      'An error was suppressed during disposal');

  async function RunTestTwoDisposeMethodsThrow() {
    try {
      TestTwoDisposeMethodsThrow();
    } catch (error) {
      assert(
          error instanceof SuppressedError,
          'error is an instanceof SuppressedError');
      assert.sameValue(error.error, firstDisposeError, 'error.error');
      assert.sameValue(
          error.suppressed, secondDisposeError, 'error.suppressed');
    }
  }
  await RunTestTwoDisposeMethodsThrow();
});
