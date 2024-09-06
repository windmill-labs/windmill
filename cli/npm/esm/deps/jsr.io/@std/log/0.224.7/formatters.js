export function jsonFormatter(logRecord) {
    return JSON.stringify({
        level: logRecord.levelName,
        datetime: logRecord.datetime.getTime(),
        message: logRecord.msg,
        args: flattenArgs(logRecord.args),
    });
}
function flattenArgs(args) {
    if (args.length === 1) {
        return args[0];
    }
    else if (args.length > 1) {
        return args;
    }
}
export const formatters = {
    jsonFormatter,
};
