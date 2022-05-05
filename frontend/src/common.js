export function pathToMeta(path) {
    const splitted = path.split('/');
    let ownerKind;
    if (splitted[0] == 'g') {
        ownerKind = 'group';
    }
    else if (splitted[0] == 'u') {
        ownerKind = 'user';
    }
    else {
        console.error('Not recognized owner:' + splitted[0]);
        return {
            ownerKind: 'user',
            owner: '',
            name: ''
        };
    }
    return {
        ownerKind,
        owner: splitted[1],
        name: splitted.slice(2).join('/')
    };
}
//# sourceMappingURL=common.js.map