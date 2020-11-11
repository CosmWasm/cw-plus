module.exports = {
  theme: 'cosmos',
  title: 'Aragon Chain Documentation',
  locales: {
    '/': {lang: 'en-US'},
  },
  base: process.env.VUEPRESS_BASE || '/',
  themeConfig: {
    repo: 'aragon/aragon-chain',
    docsRepo: 'aragon/aragon-chain',
    docsDir: 'docs',
    editLinks: true,
    // docs 1.0.168: custom true hides subpages searchbar
    // docs 1.0.168: custom true hides hub, ibc, core sidebar footer logos
    custom: true,
    logo: {
      src: '/logo.svg',
    },
    algolia: {id: 'BH4D9OD16A', key: 'ac317234e6a42074175369b2f42e9754', index: 'aragon-chain'},
    sidebar: {
      auto: false,
      nav: [
        {
          title: 'Reference',
          children: [
            {title: 'Introduction', directory: true, path: '/intro'},
            {title: 'Quick Start', directory: true, path: '/quickstart'},
            {title: 'Basics', directory: true, path: '/basics'},
            {title: 'Core Concepts', directory: true, path: '/core'},
            {title: 'Guides', directory: true, path: '/guides'}
          ]
        },
        {title: 'Specifications', children: [{title: 'Modules', directory: true, path: '/modules'}]}, {
          title: 'Resources',
          children: [
            {title: 'Aragon Chain API Reference', path: 'https://godoc.org/github.com/aragon/aragon-chain'},
            {title: 'Ethermint Docs', path: 'https://docs.ethermint.zone'},
            {title: 'Ethereum JSON RPC API Reference', path: 'https://eth.wiki/json-rpc/API'}
          ]
        }
      ]
    },
    gutter: {
      title: 'Help & Support',
      editLink: true,
      chat: {
        title: 'Developer Chat',
        text: 'Chat with Aragon Chain developers on Discord.',
        url: 'https://discord.gg/Vjw2RQ7',
        bg: 'linear-gradient(103.75deg, #1B1E36 0%, #22253F 100%)'
      },
      forum: {
        title: 'Aragon Chain Developer Forum',
        text: 'Join the Aragon Chain Developer Forum to learn more.',
        url: 'https://forum.aragon.org/c/aragon-chain',
        bg: 'linear-gradient(221.79deg, #3D6B99 -1.08%, #336699 95.88%)',
        logo: 'aragon-white'
      },
      github: {
        title: 'Found an Issue?',
        text: 'Help us improve this page by suggesting edits on GitHub.',
        url: 'https://github.com/aragon/aragon-chain/issues',
        bg: '#F8F9FC'
      }
    },
    footer: {
      logo: '/logo-bw.svg',
      textLink: {text: 'aragon.org/chain', url: 'https://aragon.org/chain'},
      services: [
        {service: 'github', url: 'https://github.com/aragon/aragon-chain'},
        {service: 'twitter', url: 'https://twitter.com/AragonProject'},
        {service: 'linkedin', url: 'https://www.linkedin.com/company/aragonproject/'},
      ],
      smallprint:
          'This website is maintained by [ChainSafe Systems](https://chainsafe.io). The contents and opinions of this website are those of Chainsafe Systems.',
      links: [
        {
          title: 'Documentation',
          children: [
            {title: 'Cosmos SDK Docs', url: 'https://docs.cosmos.network'},
            {title: 'Ethermint Docs', url: 'https://docs.ethermint.zone'},
            {title: 'Ethereum Docs', url: 'https://ethereum.org/developers'},
            {title: 'Tendermint Core Docs', url: 'https://docs.tendermint.com'}
          ]
        },
        {
          title: 'Community',
          children: [
            {title: 'Cosmos Community', url: 'https://discord.gg/W8trcGV'},
            {title: 'Aragon Chain Forum', url: 'https://forum.aragon.org/c/aragon-chain'},
            {title: 'Aragon Chain Discord', url: 'https://discord.gg/Vjw2RQ7'}
          ]
        },
        {
          title: 'Contributing',
          children: [
            {title: 'Contributing to the docs', url: 'https://github.com/aragon/aragon-chain/tree/main/docs'}, {
              title: 'Source code on GitHub',
              url: 'https://github.com/aragon/aragon-chain/blob/development/docs/DOCS_README.md'
            }
          ]
        }
      ]
    }
  },
};
