def work(items)
  items.each do |item|
    if item.ready?
      ship(item)
    end
  end
end
