/*
  Copyright (c) 2013, Oleg Grenrus
  All rights reserved.

  Redistribution and use in source and binary forms, with or without
  modification, are permitted provided that the following conditions are met:
      * Redistributions of source code must retain the above copyright
        notice, this list of conditions and the following disclaimer.
      * Redistributions in binary form must reproduce the above copyright
        notice, this list of conditions and the following disclaimer in the
        documentation and/or other materials provided with the distribution.
      * Neither the name of the Oleg Grenrus nor the
        names of its contributors may be used to endorse or promote products
        derived from this software without specific prior written permission.

  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
  ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
  WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
  DISCLAIMED. IN NO EVENT SHALL OLEG GRENRUS BE LIABLE FOR ANY
  DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
  (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
  LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
  ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
  (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
  SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
*/
(function () {
  "use strict";

  // slightly different from ES5 some, without cast to boolean
  // [x, y, z].some(f):
  // ES5:  !! ( f(x) || f(y) || f(z) || false)
  // this:    ( f(x) || f(y) || f(z) || false)
  function some(array, f) {
    var acc = false;
    for (var i = 0; i < array.length; i++) {
      acc = f(array[i], i, array);
      if (acc) {
        return acc;
      }
    }
    return acc;
  }

  function makeLexer(tokenSpecs) {
    return function (contents) {
      var tokens = [];
      var line = 1;

      function findToken() {
        return some(tokenSpecs, function (tokenSpec) {
          var m = tokenSpec.re.exec(contents);
          if (m) {
            var raw = m[0];
            contents = contents.slice(raw.length);
            return {
              raw: raw,
              matched: tokenSpec.f(m, line),
            };
          } else {
            return undefined;
          }
        });
      }

      while (contents !== "") {
        var matched = findToken();

        if (!matched) {
          var err = new SyntaxError("Unexpected character: " + contents[0] + "; input: " + contents.substr(0, 100));
          err.line = line;
          throw err;
        }

        // add line to token
        matched.matched.line = line;

        // count lines
        line += matched.raw.replace(/[^\n]/g, "").length;

        tokens.push(matched.matched);
      }

      return tokens;
    };
  }

  function fStringSingle(m) {
    // String in single quotes
    var content = m[1].replace(/([^'\\]|\\['bnrtf\\]|\\u[0-9a-fA-F]{4})/g, function (mm) {
      if (mm === "\"") {
        return "\\\"";
      } else if (mm === "\\'") {
        return "'";
      } else {
        return mm;
      }
    });

    return {
      type: "string",
      match: "\"" + content + "\"",
      value: JSON.parse("\"" + content + "\""), // abusing real JSON.parse to unquote string
    };
  }

  function fStringDouble(m) {
    return {
      type: "string",
      match: m[0],
      value: JSON.parse(m[0]),
    };
  }

  function fIdentifier(m) {
    // identifiers are transformed into strings
    return {
      type: "string",
      value: m[0],
      match: "\"" + m[0].replace(/./g, function (c) {
        return c === "\\" ? "\\\\" : c;
      }) + "\"",
    };
  }

  function fComment(m) {
    // comments are whitespace, leave only linefeeds
    return {
      type: " ",
      match: m[0].replace(/./g, function (c) {
        return (/\s/).test(c) ? c : " ";
      }),
    };
  }

  function fNumber(m) {
    return {
      type: "number",
      match: m[0],
      value: parseFloat(m[0]),
    };
  }

  function fKeyword(m) {
    var value;
    switch (m[1]) {
      case "null": value = null; break;
      case "true": value = true; break;
      case "false": value = false; break;
      // no default
    }
    return {
      type: "atom",
      match: m[0],
      value: value,
    };
  }

  function makeTokenSpecs(relaxed) {
    function f(type) {
      return function (m) {
        return { type: type, match: m[0] };
      };
    }

    var ret = [
      { re: /^\s+/, f: f(" ") },
      { re: /^\{/, f: f("{") },
      { re: /^\}/, f: f("}") },
      { re: /^\[/, f: f("[") },
      { re: /^\]/, f: f("]") },
      { re: /^,/, f: f(",") },
      { re: /^:/, f: f(":") },
      { re: /^(true|false|null)/, f: fKeyword },
      { re: /^\-?\d+(\.\d+)?([eE][+-]?\d+)?/, f: fNumber },
      { re: /^"([^"\\]|\\["bnrtf\\\/]|\\u[0-9a-fA-F]{4})*"/, f: fStringDouble },
    ];

    // additional stuff
    if (relaxed) {
      ret = ret.concat([
        { re: /^'(([^'\\]|\\['bnrtf\\\/]|\\u[0-9a-fA-F]{4})*)'/, f: fStringSingle },
        { re: /^\/\/.*?(?:\r\n|\r|\n)/, f: fComment },
        { re: /^\/\*[\s\S]*?\*\//, f: fComment },
        { re: /^[$a-zA-Z0-9_\-+\.\*\?!\|&%\^\/#\\]+/, f: fIdentifier },
      ]);
    }

    return ret;
  }

  var lexer = makeLexer(makeTokenSpecs(true));
  var strictLexer = makeLexer(makeTokenSpecs(false));

  function previousNWSToken(tokens, index) {
    for (; index >= 0; index--) {
      if (tokens[index].type !== " ") {
        return index;
      }
    }
    return undefined;
  }

  function stripTrailingComma(tokens) {
    var res = [];

    tokens.forEach(function (token, index) {
      if (token.type === "]" || token.type === "}") {
        // go backwards as long as there is whitespace, until first comma
        var commaI = previousNWSToken(res, index - 1);

        if (commaI && res[commaI].type === ",") {
          var preCommaI = previousNWSToken(res, commaI - 1);
          if (preCommaI && res[preCommaI].type !== "[" && res[preCommaI].type !== "{") {
            res[commaI] = {
              type: " ",
              match: " ",
              line: tokens[commaI].line,
            };
          }
        }
      }

      res.push(token);
    });

    return res;
  }

  function transform(text) {
    // Tokenize contents
    var tokens = lexer(text);

    // remove trailing commas
    tokens = stripTrailingComma(tokens);

    // concat stuff
    return tokens.reduce(function (str, token) {
      return str + token.match;
    }, "");
  }

  function popToken(tokens, state) {
    var token = tokens[state.pos];
    state.pos += 1;

    if (!token) {
      var line = tokens.length !== 0 ? tokens[tokens.length - 1].line : 1;
      return { type: "eof", line: line };
    }

    return token;
  }

  function strToken(token) {
    switch (token.type) {
      case "atom":
      case "string":
      case "number":
        return token.type + " " + token.match;
      case "eof":
        return "end-of-file";
      default:
        return "'" + token.type + "'";
    }
  }

  function skipColon(tokens, state) {
    var colon = popToken(tokens, state);
    if (colon.type !== ":") {
      var message = "Unexpected token: " + strToken(colon) + ", expected ':'";
      if (state.tolerant) {
        state.warnings.push({
          message: message,
          line: colon.line,
        });

        state.pos -= 1;
      } else {
        var err = new SyntaxError(message);
        err.line = colon.line;
        throw err;
      }
    }
  }

  function skipPunctuation(tokens, state, valid) {
    var punctuation = [",", ":", "]", "}"];
    var token = popToken(tokens, state);
    while (true) { // eslint-disable-line no-constant-condition
      if (valid && valid.indexOf(token.type) !== -1) {
        return token;
      } else if (token.type === "eof") {
        return token;
      } else if (punctuation.indexOf(token.type) !== -1) {
        var message = "Unexpected token: " + strToken(token) + ", expected '[', '{', number, string or atom";
        if (state.tolerant) {
          state.warnings.push({
            message: message,
            line: token.line,
          });
          token = popToken(tokens, state);
        } else {
          var err = new SyntaxError(message);
          err.line = token.line;
          throw err;
        }
      } else {
        return token;
      }
    }
  }

  function raiseError(state, token, message) {
    if (state.tolerant) {
      state.warnings.push({
        message: message,
        line: token.line,
      });
    } else {
      var err = new SyntaxError(message);
      err.line = token.line;
      throw err;
    }
  }

  function raiseUnexpected(state, token, expected) {
    raiseError(state, token, "Unexpected token: " + strToken(token) + ", expected " + expected);
  }

  function checkDuplicates(state, obj, token) {
    var key = token.value;

    if (state.duplicate && Object.prototype.hasOwnProperty.call(obj, key)) {
      raiseError(state, token, "Duplicate key: " + key);
    }
  }

  function appendPair(state, obj, key, value) {
    value = state.reviver ? state.reviver(key, value) : value;
    if (value !== undefined) {
      obj[key] = value;
    }
  }

  function parsePair(tokens, state, obj) {
    var token = skipPunctuation(tokens, state, [":"]);
    var key;
    var value;

    if (token.type !== "string") {
      raiseUnexpected(state, token, "string");
      switch (token.type) {
        case ":":
          token = {
            type: "string",
            value: "null",
            line: token.line,
          };

          state.pos -= 1;
          break;

        case "number":
        case "atom":
          token = {
            type: "string",
            value: "" + token.value,
            line: token.line,
          };
          break;

        case "[":
        case "{":
          state.pos -= 1;
          value = parseAny(tokens, state); // eslint-disable-line no-use-before-define
          appendPair(state, obj, "null", value);
          return;
        // no default
      }
    }

    checkDuplicates(state, obj, token);
    key = token.value;
    skipColon(tokens, state);
    value = parseAny(tokens, state); // eslint-disable-line no-use-before-define

    appendPair(state, obj, key, value);
  }

  function parseElement(tokens, state, arr) {
    var key = arr.length;
    var value = parseAny(tokens, state); // eslint-disable-line no-use-before-define
    arr[key] = state.reviver ? state.reviver("" + key, value) : value;
  }

  function parseObject(tokens, state) {
    return parseMany(tokens, state, {}, { // eslint-disable-line no-use-before-define
      skip: [":", "}"],
      elementParser: parsePair,
      elementName: "string",
      endSymbol: "}",
    });
  }

  function parseArray(tokens, state) {
    return parseMany(tokens, state, [], { // eslint-disable-line no-use-before-define
      skip: ["]"],
      elementParser: parseElement,
      elementName: "json object",
      endSymbol: "]",
    });
  }

  function parseMany(tokens, state, obj, opts) {
    var token = skipPunctuation(tokens, state, opts.skip);

    if (token.type === "eof") {
      raiseUnexpected(state, token, "'" + opts.endSymbol + "' or " + opts.elementName);

      token = {
        type: opts.endSymbol,
        line: token.line,
      };
    }

    switch (token.type) {
      case opts.endSymbol:
        return obj;

      default:
        state.pos -= 1; // push the token back
        opts.elementParser(tokens, state, obj);
        break;
    }

    // Rest
    while (true) { // eslint-disable-line no-constant-condition
      token = popToken(tokens, state);

      if (token.type !== opts.endSymbol && token.type !== ",") {
        raiseUnexpected(state, token, "',' or '" + opts.endSymbol + "'");

        token = {
          type: token.type === "eof" ? opts.endSymbol : ",",
          line: token.line,
        };

        state.pos -= 1;
      }

      switch (token.type) {
        case opts.endSymbol:
          return obj;

        case ",":
          opts.elementParser(tokens, state, obj);
          break;
        // no default
      }
    }
  }

  function endChecks(tokens, state, ret) {
    if (state.pos < tokens.length) {
      raiseError(state, tokens[state.pos],
        "Unexpected token: " + strToken(tokens[state.pos]) + ", expected end-of-input");
    }

    // Throw error at the end
    if (state.tolerant && state.warnings.length !== 0) {
      var message = state.warnings.length === 1 ? state.warnings[0].message : state.warnings.length + " parse warnings";
      var err = new SyntaxError(message);
      err.line = state.warnings[0].line;
      err.warnings = state.warnings;
      err.obj = ret;
      throw err;
    }
  }

  function parseAny(tokens, state, end) {
    var token = skipPunctuation(tokens, state);
    var ret;

    if (token.type === "eof") {
      raiseUnexpected(state, token, "json object");
    }

    switch (token.type) {
      case "{":
        ret = parseObject(tokens, state);
        break;
      case "[":
        ret = parseArray(tokens, state);
        break;
      case "string":
      case "number":
      case "atom":
        ret = token.value;
        break;
      // no default
    }

    if (end) {
      ret = state.reviver ? state.reviver("", ret) : ret;
      endChecks(tokens, state, ret);
    }

    return ret;
  }

  function parse(text, opts) {
    if (typeof opts === "function" || opts === undefined) {
      return JSON.parse(transform(text), opts);
    } else if (new Object(opts) !== opts) { // eslint-disable-line no-new-object
      throw new TypeError("opts/reviver should be undefined, a function or an object");
    }

    opts.relaxed = opts.relaxed !== undefined ? opts.relaxed : true;
    opts.warnings = opts.warnings || opts.tolerant || false;
    opts.tolerant = opts.tolerant || false;
    opts.duplicate = opts.duplicate || false;

    if (!opts.warnings && !opts.relaxed) {
      return JSON.parse(text, opts.reviver);
    }

    var tokens = opts.relaxed ? lexer(text) : strictLexer(text);

    if (opts.relaxed) {
      // Strip commas
      tokens = stripTrailingComma(tokens);
    }

    if (opts.warnings) {
      // Strip whitespace
      tokens = tokens.filter(function (token) {
        return token.type !== " ";
      });

      var state = { pos: 0, reviver: opts.reviver, tolerant: opts.tolerant, duplicate: opts.duplicate, warnings: [] };
      return parseAny(tokens, state, true);
    } else {
      var newtext = tokens.reduce(function (str, token) {
        return str + token.match;
      }, "");

      return JSON.parse(newtext, opts.reviver);
    }
  }

  function stringifyPair(obj, key) {
    return JSON.stringify(key) + ":" + stringify(obj[key]); // eslint-disable-line no-use-before-define
  }

  function stringify(obj) {
    switch (typeof obj) {
      case "string":
      case "number":
      case "boolean":
        return JSON.stringify(obj);
      // no default
    }
    if (Array.isArray(obj)) {
      return "[" + obj.map(stringify).join(",") + "]";
    }
    if (new Object(obj) === obj) { // eslint-disable-line no-new-object
      var keys = Object.keys(obj);
      keys.sort();
      return "{" + keys.map(stringifyPair.bind(null, obj)) + "}";
    }
    return "null";
  }

  // Export  stuff
  var RJSON = {
    transform: transform,
    parse: parse,
    stringify: stringify,
  };

  /* global window, module */
  if (typeof module !== "undefined") {
    module.exports = RJSON;
  } else if (typeof window !== "undefined") {
    window.RJSON = RJSON;
  }
}());
