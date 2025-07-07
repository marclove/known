lines:
    @find . -type f -not -path "./target/*" -not -path "./.git/*" -print0 | xargs -0 wc -l | sort -rn

coverage:
    cargo tarpaulin

commit:
    git add .
    npx llmc
    git push
