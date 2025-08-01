import * as chai from 'chai';
import { safeDump } from '../src';
import {Comment} from "../src/dumper";
const expect = chai.expect;

suite('Dumper', () => {
  suite('lineWidth dump option', () => {
    test('should respect lineWidth for multi-line strings', () => {
      const description = `Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae
Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et.`;

      expect(safeDump({ description }, { lineWidth: 100 })).to.equal(`description: >-
  Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium,
  totam rem aperiam, eaque ipsa quae

  Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed
  quia non numquam eius modi tempora incidunt ut labore et.
`);
    });

    test('should use literal block-scalar style if lineWidth is Infinity (or very lengthy)', () => {
      const description = `Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae
Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et.`;

      expect(safeDump({ description }, { lineWidth: Infinity })).to.equal(`description: |-
  Sed ut perspiciatis unde omnis iste natus error sit voluptatem accusantium doloremque laudantium, totam rem aperiam, eaque ipsa quae
  Neque porro quisquam est, qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit, sed quia non numquam eius modi tempora incidunt ut labore et.
`);
    });
  });

  test('dumps comments', () => {
    const comments: Record<string, Comment[]> = {
      '#': [
        {
          value: 'header',
          placement: 'leading',
       },
        {
          value: 'banner',
          placement: 'trailing',
       },
      ],
      '#/servers/0/url': [
        {
          value: ' first server',
          placement: 'before-eol',
        },
      ],
      '#/paths/~1roles/get': [
        {
          value: ' my comment',
          placement: 'between',
          between: ['summary', 'description'],
        },
        {
          value: ' same line',
          placement: 'before-eol',
        },
        {
          value: ' leading comment',
          placement: 'leading',
        },
      ],
      '#/paths/~1roles/get/description': [
        {
          value: ' a test description',
          placement: 'before-eol',
        },
      ],
      '#/paths/~1roles/get/responses/200': [
        {
          value: ' status code',
          placement: 'before-eol',
        },
      ],
      '#/paths/~1roles/get/summary': [
        {
          value: ' Will include all users',
          placement: 'before-eol',
        },
      ],
      '#/paths/~1roles/get/responses/200/content/application~1json/schema/items/enum': [
        {
          value: ' test',
          placement: 'between',
          between: ['0', '1'],
        },
        {
          value: ' comment',
          placement: 'between',
          between: ['0', '1'],
        },
        {
          value: ' and here',
          placement: 'trailing',
        },
        {
          value: ' start',
          placement: 'before-eol',
        },
      ],
      '#/paths/~1roles/get/responses/200/content/application~1json/schema/items/enum/0': [
        {
          value: ' first enum member',
          placement: 'before-eol',
        },
      ],
      '#/paths/~1roles/get/responses/200/content/application~1json/schema/items/enum/1': [
        {
          value: ' another one',
          placement: 'before-eol',
        },
      ],
    };

    expect(safeDump({
      "openapi": "3.0.0",
      "info": {
        "title": "Sample API",
        "version": "0.1.9"
      },
      "servers": [
        {
          "url": "http://api.stoplight.com/v1",
          "description": "Optional server description, e.g. Main (production) server"
        },
        {
          "url": "http://staging-api.example.com",
          "description": "Optional server description, e.g. Internal staging server for testing"
        }
      ],
      "paths": {
        "/roles": {
          "get": {
            "summary": "Returns a list of user roles.",
            "description": "Optional extended description in CommonMark or HTML.",
            "responses": {
              "200": {
                "description": "A JSON array of user roles",
                "content": {
                  "application/json": {
                    "schema": {
                      "type": "array",
                      "items": {
                        "enum": [
                          "user",
                          "admin"
                        ]
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }, {comments})).to.equal(`#header
openapi: 3.0.0
info:
  title: Sample API
  version: 0.1.9
servers:
  - url: 'http://api.stoplight.com/v1' # first server
    description: 'Optional server description, e.g. Main (production) server'
  - url: 'http://staging-api.example.com'
    description: 'Optional server description, e.g. Internal staging server for testing'
paths:
  /roles:
    get: # same line
      # leading comment
      summary: Returns a list of user roles. # Will include all users
      # my comment
      description: Optional extended description in CommonMark or HTML. # a test description
      responses:
        '200': # status code
          description: A JSON array of user roles
          content:
            application/json:
              schema:
                type: array
                items:
                  enum: # start
                    - user # first enum member
                    # test
                    # comment
                    - admin # another one
                    # and here
#banner
`
    );
  });

  suite('int schema', () => {
    test('binary number', () => {
      expect(safeDump({ value: '0b10101' })).to.equal(`value: '0b10101'\n`);
    });
    test('hex number', () => {
      expect(safeDump({ value: '0x25DC' })).to.equal(`value: '0x25DC'\n`);
    });
    test('oct number', () => {
      expect(safeDump({ value: '01234567' })).to.equal(`value: '01234567'\n`);
    });
    test('dec number', () => {
      expect(safeDump({ value: '1234567890' })).to.equal(`value: '1234567890'\n`);
    });
    test('leading zero dec number', () => {
      expect(safeDump({ value: '0123456789' })).to.equal(`value: '0123456789'\n`);
    });
    test('big ints', () => {
      expect(safeDump({ value: 2n ** 63n - 1n })).to.equal(`value: 9223372036854775807\n`);
      expect(safeDump({ value: 2n ** 63n * -1n })).to.equal(`value: -9223372036854775808\n`);
      expect(safeDump({ value: 2n ** 64n })).to.equal(`value: 18446744073709551616\n`);
      expect(safeDump({ value: 2n ** 128n - 1n })).to.equal(`value: 340282366920938463463374607431768211455\n`);
    });
  });

  suite('float schema', () => {
    test('with exponent', () => {
      expect(safeDump({ value: '0471120e92343' })).to.equal(`value: '0471120e92343'\n`);
    });
  });
});
