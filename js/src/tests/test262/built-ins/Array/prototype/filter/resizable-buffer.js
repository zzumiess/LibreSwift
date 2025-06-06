// Copyright 2023 the V8 project authors. All rights reserved.
// This code is governed by the BSD license found in the LICENSE file.

/*---
esid: sec-array.prototype.filter
description: >
  Array.p.filter behaves correctly on TypedArrays backed by resizable buffers.
includes: [compareArray.js, resizableArrayBufferUtils.js]
features: [resizable-arraybuffer]
---*/

for (let ctor of ctors) {
  const rab = CreateResizableArrayBuffer(4 * ctor.BYTES_PER_ELEMENT, 8 * ctor.BYTES_PER_ELEMENT);
  const fixedLength = new ctor(rab, 0, 4);
  const fixedLengthWithOffset = new ctor(rab, 2 * ctor.BYTES_PER_ELEMENT, 2);
  const lengthTracking = new ctor(rab, 0);
  const lengthTrackingWithOffset = new ctor(rab, 2 * ctor.BYTES_PER_ELEMENT);

  // Write some data into the array.
  const taWrite = new ctor(rab);
  for (let i = 0; i < 4; ++i) {
    taWrite[i] = MayNeedBigInt(taWrite, i);
  }

  // Orig. array: [0, 1, 2, 3]
  //              [0, 1, 2, 3] << fixedLength
  //                    [2, 3] << fixedLengthWithOffset
  //              [0, 1, 2, 3, ...] << lengthTracking
  //                    [2, 3, ...] << lengthTrackingWithOffset

  function isEven(n) {
    return n != undefined && Number(n) % 2 == 0;
  }
  assert.compareArray(ToNumbers(Array.prototype.filter.call(fixedLength, isEven)), [
    0,
    2
  ]);
  assert.compareArray(ToNumbers(Array.prototype.filter.call(fixedLengthWithOffset, isEven)), [2]);
  assert.compareArray(ToNumbers(Array.prototype.filter.call(lengthTracking, isEven)), [
    0,
    2
  ]);
  assert.compareArray(ToNumbers(Array.prototype.filter.call(lengthTrackingWithOffset, isEven)), [2]);

  // Shrink so that fixed length TAs go out of bounds.
  rab.resize(3 * ctor.BYTES_PER_ELEMENT);

  // Orig. array: [0, 1, 2]
  //              [0, 1, 2, ...] << lengthTracking
  //                    [2, ...] << lengthTrackingWithOffset

  assert.compareArray(Array.prototype.filter.call(fixedLength, isEven), []);
  assert.compareArray(Array.prototype.filter.call(fixedLengthWithOffset, isEven), []);

  assert.compareArray(ToNumbers(Array.prototype.filter.call(lengthTracking, isEven)), [
    0,
    2
  ]);
  assert.compareArray(ToNumbers(Array.prototype.filter.call(lengthTrackingWithOffset, isEven)), [2]);

  // Shrink so that the TAs with offset go out of bounds.
  rab.resize(1 * ctor.BYTES_PER_ELEMENT);
  assert.compareArray(Array.prototype.filter.call(fixedLength, isEven), []);
  assert.compareArray(Array.prototype.filter.call(fixedLengthWithOffset, isEven), []);
  assert.compareArray(Array.prototype.filter.call(lengthTrackingWithOffset, isEven), []);

  assert.compareArray(ToNumbers(Array.prototype.filter.call(lengthTracking, isEven)), [0]);

  // Shrink to zero.
  rab.resize(0);
  assert.compareArray(Array.prototype.filter.call(fixedLength, isEven), []);
  assert.compareArray(Array.prototype.filter.call(fixedLengthWithOffset, isEven), []);
  assert.compareArray(Array.prototype.filter.call(lengthTrackingWithOffset, isEven), []);

  assert.compareArray(ToNumbers(Array.prototype.filter.call(lengthTracking, isEven)), []);

  // Grow so that all TAs are back in-bounds.
  rab.resize(6 * ctor.BYTES_PER_ELEMENT);
  for (let i = 0; i < 6; ++i) {
    taWrite[i] = MayNeedBigInt(taWrite, i);
  }

  // Orig. array: [0, 1, 2, 3, 4, 5]
  //              [0, 1, 2, 3] << fixedLength
  //                    [2, 3] << fixedLengthWithOffset
  //              [0, 1, 2, 3, 4, 5, ...] << lengthTracking
  //                    [2, 3, 4, 5, ...] << lengthTrackingWithOffset

  assert.compareArray(ToNumbers(Array.prototype.filter.call(fixedLength, isEven)), [
    0,
    2
  ]);
  assert.compareArray(ToNumbers(Array.prototype.filter.call(fixedLengthWithOffset, isEven)), [2]);
  assert.compareArray(ToNumbers(Array.prototype.filter.call(lengthTracking, isEven)), [
    0,
    2,
    4
  ]);
  assert.compareArray(ToNumbers(Array.prototype.filter.call(lengthTrackingWithOffset, isEven)), [
    2,
    4
  ]);
}

reportCompare(0, 0);
