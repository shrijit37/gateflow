import { NextResponse } from 'next/server';

// Uniform fleet health contract: GET /health -> exactly
// { status, service, version, checks }. A static Next export has no runtime
// dependency to probe, so `assets: up` is the honest answer: the bundle
// built and this route is being served.
export async function GET() {
  return NextResponse.json({
    status: 'ok',
    service: 'gateflow-web',
    version: process.env.APP_VERSION || 'unknown',
    checks: { assets: 'up' },
  })
}