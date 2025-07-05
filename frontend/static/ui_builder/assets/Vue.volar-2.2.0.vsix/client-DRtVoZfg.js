try {
	module.exports = require('./out/nodeClientMain');
} catch {
	module.exports = require('./dist/client');
}
