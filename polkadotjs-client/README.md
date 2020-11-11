## About
This project is a stripped down vanilla JavaScript template.
This repository is only using Webpack 4 for Hot Module Reloading and to be able to include the necessary npm modules.

## Available Scripts

### `yarn install`
Installs all dependencies listed in the package.json.

### `yarn start`
Starts a Webpack development environment with Hot Module Reloading (HMR).
Open [http://localhost:4000](http://localhost:4000) to view it in the browser.
The page will reload if you make edits to any of the files.

### `yarn run build`
Builds a standalone version.


## Setup

### needs to be added to webpack config in order to load Wasm File from module
  {
    test: /\.js$/,
    // https://github.com/webpack/webpack/issues/6719#issuecomment-546840116
    loader: require.resolve('@open-wc/webpack-import-meta-loader'),
  }