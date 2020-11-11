// Our address for Alice on the dev chain
export const ALICE = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY';
export const BOB = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty';

const app = document.querySelector('#content');

export const createWrapper = (wrapperClass, headline) => {
  const div = document.createElement('div');
  const head = document.createElement('h2');
  head.textContent = headline || wrapperClass;
  div.classList.add('wrapper', wrapperClass);
  div.append(head);
  app.appendChild(div);
  return div;
};

export const createLog = (content, element = app, className) => {
  console.log(content.replace('<br />', '\n'));
  const p = document.createElement('p');
  p.classList.add('fadeIn');
  if (className) p.classList.add(className);
  p.innerHTML = content;
  element.append(p);
};

export const createError = (error, element = app) => {
  const textNode = error.type === undefined ? `Undefined error while tying to fulfill request: ${error}` : `Error of type ${error.name}:<br />${error.message}`;
  console.error(textNode.replace('<br />', '\n'));
  const p = document.createElement('p');
  p.classList.add('error');
  p.innerHTML = textNode;
  element.append(p);
};
