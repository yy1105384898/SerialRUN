# SerialRUN Website Deployment Guide

## Prerequisites

- Node.js 18+ installed
- Cloudflare account (free tier works)
- Wrangler CLI (auto-installed via npx)

## Quick Deploy

```bash
# From project root
npx wrangler pages deploy website --project-name=serialrun
```

First run will:
1. Prompt to create a new Cloudflare Pages project
2. Open browser for Cloudflare login (OAuth)
3. Ask for production branch name (default: `master`)
4. Upload files and deploy

## Subsequent Deploys

```bash
# Project already exists, just deploy
npx wrangler pages deploy website --project-name=serialrun

# Skip git dirty warning
npx wrangler pages deploy website --project-name=serialrun --commit-dirty=true
```

## URLs

After deployment, you'll get:
- **Version URL**: `https://<hash>.serialrun.pages.dev` (specific deployment)
- **Alias URL**: `https://master.serialrun.pages.dev` (latest production)

The alias URL always points to the latest deployment on the `master` branch.

## Project Structure

```
website/
├── index.html          # Landing page (hero, features, download, community)
├── guide.html          # User guide (standalone page)
├── license.html        # BSL 1.1 license explanation
├── style.css           # Global styles + responsive design
├── i18n.js             # Chinese/English translations
├── script.js           # Scroll animations
├── tux.svg             # Linux icon
├── wechat_pay_qr.jpg   # WeChat Pay QR code
├── screenshot_en.png   # English screenshot
└── screenshot_zh.png   # Chinese screenshot
```

## Custom Domain (Optional)

1. Go to Cloudflare Dashboard → Pages → serialrun → Custom domains
2. Add your domain (e.g., `serialrun.dev`)
3. Update DNS records as instructed
4. SSL is auto-provisioned

## Notes

- `website/` is in `.gitignore` — not pushed to Git repos
- Deployed independently from the code repository
- Screenshots in `website/` are copies from `assets/` — update both when changing
- i18n translations are in `i18n.js` — add new keys in both `en` and `zh` objects
- Mobile responsive CSS is in the `@media` sections at the bottom of `style.css`
