#!/bin/bash

cd target/doc
git init
echo '<meta http-equiv="refresh" content="0; url=https://dutterbutter.github.io/dkg-substrate/dkg-primitives/index.html">' > index.html
git add .
git config --global -l
git -c user.name='ci' -c user.email='ci' commit -m 'Deploy documentation'
git push -f -q https://git:${GITHUB_TOKEN}@github.com/dutterbutter/dkg-substrate HEAD:gh-pages