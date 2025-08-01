import * as YAML from '../src/'
import { AbstractVisitor } from './visitor'

import * as chai from 'chai'
const assert = chai.assert

function structure(node) {
    return new DuplicateStructureBuilder().accept(node);
}

suite('Loading a single document', () => {
    test('should work with document-end delimiters', function () {
        const input = `---
whatever: true
...`
        const doc = YAML.safeLoad(input)
        const expected_structure =
            YAML.newMap(
                [YAML.newMapping(
                    YAML.newScalar('whatever'),
                    YAML.newScalar('true'))]);

        assert.deepEqual(structure(doc), expected_structure)

        assert.lengthOf(doc.errors, 0,
            `Found error(s): ${doc.errors.toString()} when expecting none.`)
    });

    test('Document end position should be equal to input length', function () {
        const input = `
outer:
inner:
    `;
        const doc1 = YAML.load(input);
        assert.deepEqual(doc1.endPosition,input.length);
    });

    test('should store comments', () => {
        const input = `---
# first comment
mapping: # much wow
  key: value # so
  seq:
    # much discussion
    - item # wow
#comment
# much comments`;

        const doc = YAML.safeLoad(input);
        assert.deepEqual(doc.comments, [
            {
                "startPosition": 4,
                "endPosition": 19,
                "value": " first comment"
            },
            {
                "startPosition": 29,
                "endPosition": 39,
                "value": " much wow"
            },
            {
                "startPosition": 53,
                "endPosition": 57,
                "value": " so"
            },
            {
                "startPosition": 69,
                "endPosition": 86,
                "value": " much discussion"
            },
            {
                "startPosition": 98,
                "endPosition": 103,
                "value": " wow"
            },
            {
                "startPosition": 104,
                "endPosition": 112,
                "value": "comment"
            },
            {
                "startPosition": 113,
                "endPosition": 128,
                "value": " much comments"
            }
        ]);
    });
});

suite('Loading multiple documents', () => {
    test('should work with document-end delimiters', function () {
        const docs = []
        YAML.loadAll(`---
whatever: true
...
---
whatever: false
...`, d => docs.push(d))

        const expected_structure = [
            YAML.newMap(
                [YAML.newMapping(
                    YAML.newScalar('whatever'),
                    YAML.newScalar('true'))]),
            YAML.newMap(
                [YAML.newMapping(
                    YAML.newScalar('whatever'),
                    YAML.newScalar('false'))])
        ];

        assert.deepEqual(docs.map(d => structure(d)), expected_structure)

        docs.forEach(doc =>
            assert.lengthOf(doc.errors, 0,
                `Found error(s): ${doc.errors.toString()} when expecting none.`))
    });

    test('Last document end position should be equal to input length', function () {
        const input = `
outer1:
inner1:
...
---
outer2:
inner2:
    `;
        const documents: YAML.YAMLDocument[] = [];
        YAML.loadAll(input,x=>documents.push(x));
        const doc2 = documents[1];
        assert.deepEqual(doc2.endPosition,input.length);
    });
});

class DuplicateStructureBuilder extends AbstractVisitor {
    visitScalar(node: YAML.YAMLScalar) {
        return YAML.newScalar(node.value)
    }
    visitMapping(node: YAML.YAMLMapping) {
        return YAML.newMapping(this.visitScalar(node.key), this.accept(node.value))
    }
    visitSequence(node: YAML.YAMLSequence) {
        const seq = YAML.newSeq()
        seq.items = node.items.map(n => this.accept(n))
        return seq
    }
    visitMap(node: YAML.YamlMap) {
        return YAML.newMap(node.mappings.map(n => this.accept(n)));
    }
    visitAnchorRef(node: YAML.YAMLAnchorReference) {
        throw new Error("Method not implemented.");
    }
    visitIncludeRef(node: YAML.YAMLNode) {
        throw new Error("Method not implemented.");
    }
}
