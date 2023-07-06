#!/usr/bin/env node
"use strict";

var program = require("commander");
var chalk = require("chalk");
var fs = require("fs");
var path = require("path");

var rjson = require("../relaxed-json.js");

var pkgJson = JSON.parse(fs.readFileSync(path.join(__dirname, "..", "package.json")).toString());

program.usage("[options] file.js");
program.version(pkgJson.version);
program.option("--no-relaxed", "Parse as strict JSON");
program.option("--no-warnings", "Don't emit precise (use JSON.parse)");
program.option("--no-duplicates", "Don't warn on duplicate keys");

function cli(argv) {
  program.parse(argv);

  if (program.args.length !== 1) {
    console.error("Error: input file is required");
    console.log(program.help());
    return 1;
  }

  var opts = {
    relaxed: program.relaxed,
    warnings: program.warnings,
    duplicates: program.duplicates,
  };

  var contents = fs.readFileSync(program.args[0]).toString();

  try {
    var json = rjson.parse(contents, opts);
    console.log(JSON.stringify(json, null, 2));
  } catch (e) {
    if (e instanceof SyntaxError && typeof e.line === "number") {
      console.error("Error on line " + e.line + ": " + e.message);
      console.log(chalk.grey(contents.split(/\n/)[e.line - 1]));
    }
  }

  return 0;
}

var ret = cli(process.argv);
/* eslint-disable no-process-exit */
process.exit(ret);
/* eslint-enable no-process-exit */
