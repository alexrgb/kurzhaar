declare module '@kurzhaar/protocol' {
  export function decode(body: string): {
    key?: string;
    screen?: string;
    lang?: string;
    identity?: string;
    events: {
      ts: number;
      type: number;
      name?: string;
      url?: string;
      title?: string;
      referrer?: string;
      props: Record<string, string>;
    }[];
  };
}
