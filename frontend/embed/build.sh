npm run build-embed
cp static/Inter-Variable.ttf dist
cp static/tailwind.js dist
npx tailwindcss -o src/lib/assets/app.css --minify -o dist/tailwind.css
cp embed/index.html dist