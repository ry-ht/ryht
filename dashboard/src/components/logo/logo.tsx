import type { LinkProps } from '@mui/material/Link';

import { useId } from 'react';
import { mergeClasses } from 'minimal-shared/utils';

import Link from '@mui/material/Link';
import { styled, useTheme } from '@mui/material/styles';

import { RouterLink } from 'src/routes/components';

import { logoClasses } from './classes';

// ----------------------------------------------------------------------

export type LogoProps = LinkProps & {
  isSingle?: boolean;
  disabled?: boolean;
};

export function Logo({
  sx,
  disabled,
  className,
  href = '/',
  isSingle = true,
  ...other
}: LogoProps) {
  const theme = useTheme();

  const uniqueId = useId();

  const TEXT_PRIMARY = theme.vars.palette.text.primary;
  const PRIMARY_LIGHT = theme.vars.palette.primary.light;
  const PRIMARY_MAIN = theme.vars.palette.primary.main;
  const PRIMARY_DARKER = theme.vars.palette.primary.dark;

  /*
    * OR using local (public folder)
    *
    const singleLogo = (
      <img
        alt="Single logo"
        src={`${CONFIG.assetsDir}/logo/logo-single.svg`}
        width="100%"
        height="100%"
      />
    );

    const fullLogo = (
      <img
        alt="Full logo"
        src={`${CONFIG.assetsDir}/logo/logo-full.svg`}
        width="100%"
        height="100%"
      />
    );
    *
    */

  const singleLogo = (
    <svg
      width="100%"
      height="100%"
      viewBox="0 0 512 512"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        <linearGradient
          id={`${uniqueId}-core`}
          x1="256"
          y1="156"
          x2="256"
          y2="356"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor={PRIMARY_LIGHT} />
          <stop offset="0.5" stopColor={PRIMARY_MAIN} />
          <stop offset="1" stopColor={PRIMARY_DARKER} />
        </linearGradient>
        <linearGradient
          id={`${uniqueId}-ring`}
          x1="256"
          y1="76"
          x2="256"
          y2="436"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor={PRIMARY_DARKER} stopOpacity="0.3" />
          <stop offset="1" stopColor={PRIMARY_MAIN} stopOpacity="0.6" />
        </linearGradient>
        <linearGradient
          id={`${uniqueId}-agent`}
          x1="0"
          y1="0"
          x2="1"
          y2="1"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor={PRIMARY_MAIN} />
          <stop offset="1" stopColor={PRIMARY_LIGHT} />
        </linearGradient>
      </defs>

      {/* Outer coordination ring */}
      <circle
        cx="256"
        cy="256"
        r="180"
        stroke={`url(#${uniqueId}-ring)`}
        strokeWidth="2"
        fill="none"
        opacity="0.4"
      />

      {/* Connection lines from center to agents */}
      <g stroke={PRIMARY_MAIN} strokeWidth="1.5" opacity="0.3">
        <line x1="256" y1="256" x2="256" y2="76" />
        <line x1="256" y1="256" x2="383.14" y2="128.86" />
        <line x1="256" y1="256" x2="436" y2="256" />
        <line x1="256" y1="256" x2="383.14" y2="383.14" />
        <line x1="256" y1="256" x2="256" y2="436" />
        <line x1="256" y1="256" x2="128.86" y2="383.14" />
        <line x1="256" y1="256" x2="76" y2="256" />
        <line x1="256" y1="256" x2="128.86" y2="128.86" />
      </g>

      {/* Central core (Cortex cognitive system) */}
      <circle
        cx="256"
        cy="256"
        r="80"
        fill={`url(#${uniqueId}-core)`}
      />

      {/* Inner cognitive pattern */}
      <g opacity="0.3">
        <circle cx="256" cy="230" r="18" fill={PRIMARY_LIGHT} />
        <circle cx="280" cy="256" r="16" fill={PRIMARY_LIGHT} />
        <circle cx="256" cy="282" r="18" fill={PRIMARY_LIGHT} />
        <circle cx="232" cy="256" r="16" fill={PRIMARY_LIGHT} />
      </g>

      {/* Core border */}
      <circle
        cx="256"
        cy="256"
        r="80"
        stroke={PRIMARY_DARKER}
        strokeWidth="3"
        fill="none"
      />

      {/* 8 Agent nodes positioned around the center */}
      {/* Agent 1 - Top (Developer) */}
      <circle cx="256" cy="76" r="24" fill={`url(#${uniqueId}-agent)`} />
      <circle cx="256" cy="76" r="24" stroke={PRIMARY_DARKER} strokeWidth="2" fill="none" />

      {/* Agent 2 - Top-Right (Reviewer) */}
      <circle cx="383.14" cy="128.86" r="22" fill={`url(#${uniqueId}-agent)`} />
      <circle cx="383.14" cy="128.86" r="22" stroke={PRIMARY_DARKER} strokeWidth="2" fill="none" />

      {/* Agent 3 - Right (Tester) */}
      <circle cx="436" cy="256" r="24" fill={`url(#${uniqueId}-agent)`} />
      <circle cx="436" cy="256" r="24" stroke={PRIMARY_DARKER} strokeWidth="2" fill="none" />

      {/* Agent 4 - Bottom-Right (Documenter) */}
      <circle cx="383.14" cy="383.14" r="22" fill={`url(#${uniqueId}-agent)`} />
      <circle cx="383.14" cy="383.14" r="22" stroke={PRIMARY_DARKER} strokeWidth="2" fill="none" />

      {/* Agent 5 - Bottom (Architect) */}
      <circle cx="256" cy="436" r="24" fill={`url(#${uniqueId}-agent)`} />
      <circle cx="256" cy="436" r="24" stroke={PRIMARY_DARKER} strokeWidth="2" fill="none" />

      {/* Agent 6 - Bottom-Left (Researcher) */}
      <circle cx="128.86" cy="383.14" r="22" fill={`url(#${uniqueId}-agent)`} />
      <circle cx="128.86" cy="383.14" r="22" stroke={PRIMARY_DARKER} strokeWidth="2" fill="none" />

      {/* Agent 7 - Left (Optimizer) */}
      <circle cx="76" cy="256" r="24" fill={`url(#${uniqueId}-agent)`} />
      <circle cx="76" cy="256" r="24" stroke={PRIMARY_DARKER} strokeWidth="2" fill="none" />

      {/* Agent 8 - Top-Left (Orchestrator) */}
      <circle cx="128.86" cy="128.86" r="22" fill={`url(#${uniqueId}-agent)`} />
      <circle cx="128.86" cy="128.86" r="22" stroke={PRIMARY_DARKER} strokeWidth="2" fill="none" />
    </svg>
  );

  const fullLogo = (
    <svg
      width="100%"
      height="100%"
      viewBox="0 0 420 128"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        <linearGradient
          id={`${uniqueId}-full-core`}
          x1="64"
          y1="24"
          x2="64"
          y2="104"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor={PRIMARY_LIGHT} />
          <stop offset="0.5" stopColor={PRIMARY_MAIN} />
          <stop offset="1" stopColor={PRIMARY_DARKER} />
        </linearGradient>
        <linearGradient
          id={`${uniqueId}-full-ring`}
          x1="64"
          y1="0"
          x2="64"
          y2="128"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor={PRIMARY_DARKER} stopOpacity="0.3" />
          <stop offset="1" stopColor={PRIMARY_MAIN} stopOpacity="0.6" />
        </linearGradient>
        <linearGradient
          id={`${uniqueId}-full-agent`}
          x1="0"
          y1="0"
          x2="1"
          y2="1"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor={PRIMARY_MAIN} />
          <stop offset="1" stopColor={PRIMARY_LIGHT} />
        </linearGradient>
      </defs>

      {/* Logo part - simplified single logo */}
      <g>
        {/* Outer ring */}
        <circle
          cx="64"
          cy="64"
          r="54"
          stroke={`url(#${uniqueId}-full-ring)`}
          strokeWidth="1.5"
          fill="none"
          opacity="0.4"
        />

        {/* Connection lines */}
        <g stroke={PRIMARY_MAIN} strokeWidth="1" opacity="0.25">
          <line x1="64" y1="64" x2="64" y2="10" />
          <line x1="64" y1="64" x2="102.2" y2="25.8" />
          <line x1="64" y1="64" x2="118" y2="64" />
          <line x1="64" y1="64" x2="102.2" y2="102.2" />
          <line x1="64" y1="64" x2="64" y2="118" />
          <line x1="64" y1="64" x2="25.8" y2="102.2" />
          <line x1="64" y1="64" x2="10" y2="64" />
          <line x1="64" y1="64" x2="25.8" y2="25.8" />
        </g>

        {/* Central core */}
        <circle
          cx="64"
          cy="64"
          r="24"
          fill={`url(#${uniqueId}-full-core)`}
        />

        {/* Inner pattern */}
        <g opacity="0.3">
          <circle cx="64" cy="54" r="5" fill={PRIMARY_LIGHT} />
          <circle cx="72" cy="64" r="4" fill={PRIMARY_LIGHT} />
          <circle cx="64" cy="74" r="5" fill={PRIMARY_LIGHT} />
          <circle cx="56" cy="64" r="4" fill={PRIMARY_LIGHT} />
        </g>

        {/* Core border */}
        <circle
          cx="64"
          cy="64"
          r="24"
          stroke={PRIMARY_DARKER}
          strokeWidth="2"
          fill="none"
        />

        {/* 8 Agents - smaller nodes */}
        <circle cx="64" cy="10" r="7" fill={`url(#${uniqueId}-full-agent)`} />
        <circle cx="64" cy="10" r="7" stroke={PRIMARY_DARKER} strokeWidth="1.5" fill="none" />

        <circle cx="102.2" cy="25.8" r="6" fill={`url(#${uniqueId}-full-agent)`} />
        <circle cx="102.2" cy="25.8" r="6" stroke={PRIMARY_DARKER} strokeWidth="1.5" fill="none" />

        <circle cx="118" cy="64" r="7" fill={`url(#${uniqueId}-full-agent)`} />
        <circle cx="118" cy="64" r="7" stroke={PRIMARY_DARKER} strokeWidth="1.5" fill="none" />

        <circle cx="102.2" cy="102.2" r="6" fill={`url(#${uniqueId}-full-agent)`} />
        <circle cx="102.2" cy="102.2" r="6" stroke={PRIMARY_DARKER} strokeWidth="1.5" fill="none" />

        <circle cx="64" cy="118" r="7" fill={`url(#${uniqueId}-full-agent)`} />
        <circle cx="64" cy="118" r="7" stroke={PRIMARY_DARKER} strokeWidth="1.5" fill="none" />

        <circle cx="25.8" cy="102.2" r="6" fill={`url(#${uniqueId}-full-agent)`} />
        <circle cx="25.8" cy="102.2" r="6" stroke={PRIMARY_DARKER} strokeWidth="1.5" fill="none" />

        <circle cx="10" cy="64" r="7" fill={`url(#${uniqueId}-full-agent)`} />
        <circle cx="10" cy="64" r="7" stroke={PRIMARY_DARKER} strokeWidth="1.5" fill="none" />

        <circle cx="25.8" cy="25.8" r="6" fill={`url(#${uniqueId}-full-agent)`} />
        <circle cx="25.8" cy="25.8" r="6" stroke={PRIMARY_DARKER} strokeWidth="1.5" fill="none" />
      </g>

      {/* Text "RyHt" */}
      <g fill={TEXT_PRIMARY}>
        {/* R */}
        <path d="M 160 40 L 160 90 M 160 40 L 180 40 Q 190 40 190 50 Q 190 60 180 60 L 160 60 M 175 60 L 190 90" strokeWidth="6" stroke={TEXT_PRIMARY} strokeLinecap="round" strokeLinejoin="round" />

        {/* y */}
        <path d="M 210 55 L 210 95 Q 210 105 200 105 M 230 55 L 230 75 M 210 75 L 230 75" strokeWidth="6" stroke={TEXT_PRIMARY} strokeLinecap="round" strokeLinejoin="round" />

        {/* H */}
        <path d="M 250 40 L 250 90 M 270 40 L 270 90 M 250 65 L 270 65" strokeWidth="6" stroke={TEXT_PRIMARY} strokeLinecap="round" strokeLinejoin="round" />

        {/* t */}
        <path d="M 290 45 L 290 85 Q 290 90 295 90 L 305 90 M 280 55 L 300 55" strokeWidth="6" stroke={TEXT_PRIMARY} strokeLinecap="round" strokeLinejoin="round" />
      </g>
    </svg>
  );

  return (
    <LogoRoot
      component={RouterLink}
      href={href}
      aria-label="Logo"
      underline="none"
      className={mergeClasses([logoClasses.root, className])}
      sx={[
        {
          width: 40,
          height: 40,
          ...(!isSingle && { width: 118, height: 36 }),
          ...(disabled && { pointerEvents: 'none' }),
        },
        ...(Array.isArray(sx) ? sx : [sx]),
      ]}
      {...other}
    >
      {isSingle ? singleLogo : fullLogo}
    </LogoRoot>
  );
}

// ----------------------------------------------------------------------

const LogoRoot = styled(Link)(() => ({
  flexShrink: 0,
  color: 'transparent',
  display: 'inline-flex',
  verticalAlign: 'middle',
}));
