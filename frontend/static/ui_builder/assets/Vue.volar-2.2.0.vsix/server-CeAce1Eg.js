try {
	module.exports = require('@vue/language-server/bin/vue-language-server');
} catch {
	module.exports = require('./dist/server');
}
