module Demo
  class Worker
    def run(items)
      choose = ->(item) { item if item && ready? || forced? }
      items.each do |item|
        next unless item
        choose.call(item)
      end
    end

    def self.empty
      []
    end
  end
end
