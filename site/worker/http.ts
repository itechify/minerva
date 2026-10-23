// What a route answers: a status and a JSON body, and any headers and cookies of its own.
export interface Answer {
  status: number;
  body?: unknown;
  headers?: Record<string, string>;
  /** `Set-Cookie` values, one header each. */
  cookies?: string[];
}

export function json(answer: Answer, cacheControl: string): Response {
  const cache = answer.status === 200 ? cacheControl : "no-store";
  const headers = new Headers({ "cache-control": cache, ...answer.headers });
  for (const cookie of answer.cookies ?? []) headers.append("set-cookie", cookie);
  if (answer.body === undefined) return new Response(null, { status: answer.status, headers });
  return Response.json(answer.body, { status: answer.status, headers });
}

/** A request the API can't take: `detail` says why. */
export function badRequest(detail: string): Answer {
  return { status: 400, body: { error: "bad-request", detail } };
}

/** A redirect to `location`, a path on the site or an address elsewhere. */
export function redirect(location: string, cookies?: string[]): Answer {
  return { status: 302, headers: { location }, cookies };
}

/** The request's JSON body, or `undefined` if it isn't JSON. */
export async function body(request: Request): Promise<unknown> {
  try {
    return await request.json();
  } catch {
    return undefined;
  }
}
