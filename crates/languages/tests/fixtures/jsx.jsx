class View {
  render() {
    const select = (ready) => ready ? <strong>yes</strong> : <span>no</span>;
    return <main>{this.props.ready && select(true)}</main>;
  }
}
