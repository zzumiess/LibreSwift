// |reftest| shell-option(--enable-temporal) skip-if(!this.hasOwnProperty('Temporal')||!xulRuntime.shell) -- Temporal is not enabled unconditionally, requires shell-options
// Copyright (C) 2022 Igalia, S.L. All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.

/*---
esid: sec-temporal.plaindatetime.from
description: A number as calendar in a property bag is not accepted
features: [Temporal]
---*/

const numbers = [
  1,
  19970327,
  -19970327,
  1234567890,
];

for (const calendar of numbers) {
  const arg = { year: 1976, monthCode: "M11", day: 18, calendar };
  assert.throws(
    TypeError,
    () => Temporal.PlainDateTime.from(arg),
    "Numbers cannot be used as a calendar"
  );
}

reportCompare(0, 0);
