class View {
  render(): JSX.Element {
    const select = (ready: boolean) => ready ? <b>yes</b> : <i>no</i>;
    return <main>{this.props.ready || select(false)}</main>;
  }
}
