// deno-fmt-ignore-file
// deno-lint-ignore-file

// Copyright Joyent and Node contributors. All rights reserved. MIT license.
// Taken from Node 18.12.1
// This file is automatically generated by "node/_tools/setup.ts". Do not modify this file manually

'use strict';

require('../common');
const assert = require('assert');

assert.throws(() => new Buffer(42, 'utf8'), {
  code: 'ERR_INVALID_ARG_TYPE',
  name: 'TypeError',
  message: 'The "string" argument must be of type string. Received type ' +
           'number (42)'
});