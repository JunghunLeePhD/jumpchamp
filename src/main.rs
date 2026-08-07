fn find_primes(limit: usize) -> Vec<usize> {
    if limit < 2 {
        return vec![];
    }

    let mut is_prime = vec![true; limit + 1];
    is_prime[0] = false;
    is_prime[1] = false;

    let mut p = 2;
    while p * p <= limit {
        if is_prime[p] {
            let mut i = p * p;
            while i <= limit {
                is_prime[i] = false;
                i += p;
            }
        }
        p += 1;
    }

    (2..=limit).filter(|&x| is_prime[x]).collect()
}

fn main() {
    let limit = 100; 
    let primes = find_primes(limit);

    println!("Found {} primes up to {}:", primes.len(), limit);
    println!("{:?}", primes);
}