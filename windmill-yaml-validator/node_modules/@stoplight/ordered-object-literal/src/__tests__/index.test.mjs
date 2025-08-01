/* eslint-disable sort-keys */
import createOrderedObj, {
  deserialize,
  serialize,
  setOrder,
  ORDER_KEY_ID,
} from '../index.mjs';

describe('Ordered object literal', () => {
  describe('set trap', () => {
    it('given missing property, adds it', () => {
      const obj = createOrderedObj({ c: false });
      obj.x = true;
      expect(Object.keys(obj)).to.deep.equal(['c', 'x']);
      expect(obj).to.deep.equal({ c: false, x: true });
    });

    it('given missing own property, adds it', () => {
      const obj = createOrderedObj({});
      obj.toString = 'abc';
      expect(Object.keys(obj)).to.deep.equal(['toString']);
      expect(obj).to.deep.equal({ toString: 'abc' });
    });

    it('given existing property, overwrites it', () => {
      const obj = createOrderedObj({ x: false });
      obj.x = true;
      expect(obj).to.have.keys(['x']);
      expect(obj).to.deep.equal({ x: true });
    });

    it('given numeric key, retains its real position', () => {
      const obj = createOrderedObj({ x: true });
      obj['0'] = true;
      expect(Object.keys(obj)).to.deep.equal(['x', '0']);
      expect(obj).to.deep.equal({ 0: true, x: true });
    });

    it('given existing non-extensible, throws', () => {
      const target = Object.preventExtensions({});

      const obj = createOrderedObj(target);
      expect(() => void (obj.d = true)).to.throw(TypeError);
      expect(obj).not.to.have.property('d');
    });
  });

  describe('delete trap', () => {
    it('given missing property, does nothing', () => {
      const obj = createOrderedObj({ a: 0 });
      expect(delete obj.x).to.be.true;
      expect(obj).to.have.keys(['a']);
      expect(obj).to.deep.equal({ a: 0 });
    });

    it('given existing property, removes it', () => {
      const obj = createOrderedObj({ a: 0, b: 0, c: 0 });
      expect(delete obj.b).to.be.true;
      expect(obj).not.to.have.property('b');
      expect(obj).to.have.keys(['a', 'c']);
      expect(obj[Symbol.for(ORDER_KEY_ID)]).to.deep.equal([
        'a',
        'c',
        Symbol.for(ORDER_KEY_ID),
      ]);
      expect(obj).to.deep.equal({ a: 0, c: 0 });
    });

    it('given existing non-configurable property, throws', () => {
      const target = { a: 0, c: 0 };
      Reflect.defineProperty(target, 'b', {
        enumerable: true,
        value: 0,
      });

      const obj = createOrderedObj(target);
      expect(() => void delete obj.b).to.throw(TypeError);
      expect(obj).to.have.keys(['a', 'b', 'c']);
      expect(obj).to.deep.equal({ a: 0, b: 0, c: 0 });
    });
  });

  describe('ownKeys trap', () => {
    it('returns own string keys', () => {
      const obj = createOrderedObj({ x: true });
      expect(Object.keys(obj)).to.deep.equal(['x']);
      expect(obj).to.deep.equal({ x: true });
    });

    it('returns own symbol keys', () => {
      const symbolX = Symbol('x');
      const obj = createOrderedObj({ [symbolX]: 0 });
      expect(Object.getOwnPropertySymbols(obj)).to.deep.equal([
        symbolX,
        Symbol.for(ORDER_KEY_ID),
      ]);
    });

    it('returns own keys', () => {
      const symbolX = Symbol('x');
      const obj = createOrderedObj({ x: false, [symbolX]: 0 });
      expect(Reflect.ownKeys(obj)).to.deep.equal([
        'x',
        symbolX,
        Symbol.for(ORDER_KEY_ID),
      ]);
    });
  });

  describe('defineProperty trap', () => {
    it('defines new property', () => {
      const obj = createOrderedObj({ x: true });
      expect(Reflect.defineProperty(obj, 'foo', {})).to.be.true;
      expect(Reflect.defineProperty(obj, 'toString', { enumerable: true })).to
        .be.true;
      expect(Object.keys(obj)).to.deep.equal(['x', 'toString']);
    });

    it('given numeric key, retains its real position', () => {
      const obj = createOrderedObj({ x: true });
      Reflect.defineProperty(obj, '0', { enumerable: true });
      expect(Object.keys(obj)).to.deep.equal(['x', '0']);
    });

    it('handles recreation of already trapped object', () => {
      const obj = createOrderedObj({ b: 0 });
      const newObj = createOrderedObj({ ...obj }, Reflect.ownKeys(obj));
      expect(newObj).to.have.keys(['b']);
      expect(newObj).to.deep.equal({ b: 0 });
    });
  });

  describe('(de)serialization', () => {
    it('works', () => {
      const obj = createOrderedObj({ x: false, ['0']: 'boo' });
      setOrder(obj, ['x', '0']);
      obj.c = createOrderedObj({ '2': 0, b: false, '1': 'x' });
      setOrder(obj.c, ['2', 'b', '1']);

      const deserializedObj = deserialize(
        JSON.parse(JSON.stringify(serialize(obj, true))),
        true,
      );

      expect(Object.keys(deserializedObj)).to.deep.equal(['x', '0', 'c']);
      expect(Object.keys(deserializedObj.c)).to.deep.equal(['2', 'b', '1']);

      expect(deserializedObj[Symbol.for(ORDER_KEY_ID)]).to.deep.equal([
        'x',
        '0',
        'c',
        Symbol.for(ORDER_KEY_ID),
      ]);
    });

    it('supports arrays', () => {
      const obj = createOrderedObj(
        {
          x: [
            0,
            createOrderedObj(
              {
                [0]: false,
                foo: true,
                bar: [1, createOrderedObj({ c: true }, ['c'])],
              },
              ['0', 'foo', 'bar'],
            ),
          ],
        },
        ['x'],
      );

      const deserializedObj = deserialize(
        JSON.parse(JSON.stringify(serialize(obj, true))),
        true,
      );

      expect(Object.keys(deserializedObj.x[1])).to.deep.equal([
        '0',
        'foo',
        'bar',
      ]);
    });
  });

  describe('when array is given as input', () => {
    afterEach(() => {
      delete process.env.NODE_ENV;
    });

    it('given test environment, throws', () => {
      process.env.NODE_ENV = 'test';

      expect(createOrderedObj.bind(void 0, [])).to.throw();
    });

    it('given development environment, throws', () => {
      process.env.NODE_ENV = 'development';

      expect(createOrderedObj.bind(void 0, [])).to.throw();
    });

    it('given production environment, does not throw', () => {
      process.env.NODE_ENV = 'production';

      expect(createOrderedObj.bind(void 0, [])).not.to.throw();
    });

    it('given no environment set, does not throw', () => {
      expect(createOrderedObj.bind(void 0, [])).not.to.throw();
    });
  });

  it('survives freezing', () => {
    const obj = createOrderedObj({ 0: 'c', b: 0 }, ['0', 'b']);
    Object.freeze(obj);
    expect(obj).to.deep.equal({ 0: 'c', b: 0 });
    expect(Object.keys(obj)).to.deep.equal(['0', 'b']);
  });

  it('survives sealing', () => {
    const obj = createOrderedObj({ 0: 'c', b: 0 }, ['0', 'b']);
    Object.seal(obj);
    expect(obj).to.deep.equal({ 0: 'c', b: 0 });
    expect(Object.keys(obj)).to.deep.equal(['0', 'b']);
  });

  it('survives preventExtensions', () => {
    const obj = createOrderedObj({ 0: 'c', b: 0 }, ['0', 'b']);
    Object.preventExtensions(obj);
    expect(obj).to.deep.equal({ 0: 'c', b: 0 });
    expect(Object.keys(obj)).to.deep.equal(['0', 'b']);
  });
});
