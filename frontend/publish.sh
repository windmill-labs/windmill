npm run package
sed -i -e 's/windmill/windmill-components/g' package.json
npm publish
sed -i -e 's/windmill-components/windmill/g' package.json
