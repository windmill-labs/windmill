/**
 * Compile the given `.gitignore` content (not filename!)
 * and return an object with `accepts`, `denies` and `maybe` methods.
 * These methods each accepts a single filename and determines whether
 * they are acceptable or unacceptable according to the `.gitignore` definition.
 *
 *
 * @param  {String} content The `.gitignore` content to compile.
 * @return {Object}         The helper object with methods that operate on the compiled content.
 */
exports.compile = function (content) {
  var parsed = exports.parse(content),
      positives = parsed[0],
      negatives = parsed[1];
  return {
    accepts: function (input) {
      if (input[0] === '/') input = input.slice(1);
      return negatives[0].test(input) || !positives[0].test(input);
    },
    denies: function (input) {
      if (input[0] === '/') input = input.slice(1);
      return !(negatives[0].test(input) || !positives[0].test(input));
    },
    maybe: function (input) {
      if (input[0] === '/') input = input.slice(1);
      return negatives[1].test(input) || !positives[1].test(input);
    }
  };
};

/**
 * Parse the given `.gitignore` content and return an array
 * containing two further arrays - positives and negatives.
 * Each of these two arrays in turn contains two regexps, one
 * strict and one for 'maybe'.
 *
 * @param  {String} content  The content to parse,
 * @return {Array[]}         The parsed positive and negatives definitions.
 */
exports.parse = function (content) {
  return content.split('\n')
  .map(function (line) {
    line = line.trim();
    return line;
  })
  .filter(function (line) {
    return line && line[0] !== '#';
  })
  .reduce(function (lists, line) {
    var isNegative = line[0] === '!';
    if (isNegative) {
      line = line.slice(1);
    }
    if (line[0] === '/')
      line = line.slice(1);
    if (isNegative) {
      lists[1].push(line);
    }
    else {
      lists[0].push(line);
    }
    return lists;
  }, [[], []])
  .map(function (list) {
    return list
    .sort()
    .map(prepareRegexes)
    .reduce(function (list, prepared) {
      list[0].push(prepared[0]);
      list[1].push(prepared[1]);
      return list;
    }, [[], [], []]);
  })
  .map(function (item) {
    return [
      item[0].length > 0 ? new RegExp('^((' + item[0].join(')|(') + '))') : new RegExp('$^'),
      item[1].length > 0 ? new RegExp('^((' + item[1].join(')|(') + '))') : new RegExp('$^')
    ]
  });
};

function prepareRegexes (pattern) {
  return [
    // exact regex
    prepareRegexPattern(pattern),
    // partial regex
    preparePartialRegex(pattern)
  ];
};

function prepareRegexPattern (pattern) {
  return escapeRegex(pattern).replace('**', '(.+)').replace('*', '([^\\/]+)');
}

function preparePartialRegex (pattern) {
  return pattern
  .split('/')
  .map(function (item, index) {
    if (index)
      return '([\\/]?(' + prepareRegexPattern(item) + '\\b|$))';
    else
      return '(' + prepareRegexPattern(item) + '\\b)';
  })
  .join('');
}

function escapeRegex (pattern) {
  return pattern.replace(/[\-\[\]\/\{\}\(\)\+\?\.\\\^\$\|]/g, "\\$&");
}
