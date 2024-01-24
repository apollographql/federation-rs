// @ts-nocheck

const assertEq = (a: unknown, b: unknown) => {
  if (a !== b) {
    throw `${a} is not equal to ${b}`;
  }
};

const url = new URL("https://www.test.com/test2");

assertEq("/test2", url.pathname);
assertEq("www.test.com", url.hostname);
assertEq("https", url.scheme);
