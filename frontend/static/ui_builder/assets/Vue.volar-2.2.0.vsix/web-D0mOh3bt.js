const vscode = require("vscode");

exports.activate = async function () {
	const tsWeb = vscode.extensions.getExtension('johnsoncodehk.vscode-typescript-web');
	if (!tsWeb) {
		const select = await vscode.window.showInformationMessage(
			'Install "TypeScript IntelliSense for Web" to enable Vue IntelliSense.',
			'Install',
			'Cancel',
		);
		if (select === 'Install') {
			vscode.commands.executeCommand('workbench.extensions.search', 'johnsoncodehk.vscode-typescript-web');
		}
	}
};
